use std::fmt;
use std::mem::{offset_of, size_of};

use windows_sys::Wdk::System::SystemInformation::{
    NtQuerySystemInformation, SystemProcessInformation,
};
use windows_sys::Win32::Foundation::{
    NTSTATUS, STATUS_BUFFER_TOO_SMALL, STATUS_INFO_LENGTH_MISMATCH,
};
use windows_sys::Win32::System::WindowsProgramming::SYSTEM_PROCESS_INFORMATION;

// x64 layout of SYSTEM_PROCESS_INFORMATION. winternl.h names only some fields and marks
// the rest `Reserved`; the other offsets follow the commonly documented layout that the
// Phase 0 spike validated against live data. The assertions below pin every offset the
// public header does name.
const ENTRY_SIZE: usize = 256;
const NEXT_ENTRY_OFFSET: usize = 0;
const WORKING_SET_PRIVATE_SIZE: usize = 8;
const CYCLE_TIME: usize = 24;
const CREATE_TIME: usize = 32;
const USER_TIME: usize = 40;
const KERNEL_TIME: usize = 48;
const IMAGE_NAME_LENGTH: usize = 56;
const IMAGE_NAME_BUFFER: usize = 64;
const UNIQUE_PROCESS_ID: usize = 80;
const WORKING_SET_SIZE: usize = 144;
const PAGEFILE_USAGE: usize = 184;
const READ_TRANSFER_COUNT: usize = 232;
const WRITE_TRANSFER_COUNT: usize = 240;
const OTHER_TRANSFER_COUNT: usize = 248;

const _: () = {
    type Spi = SYSTEM_PROCESS_INFORMATION;
    assert!(size_of::<Spi>() == ENTRY_SIZE);
    assert!(offset_of!(Spi, NextEntryOffset) == NEXT_ENTRY_OFFSET);
    assert!(offset_of!(Spi, ImageName) == IMAGE_NAME_LENGTH);
    assert!(offset_of!(Spi, ImageName) + 8 == IMAGE_NAME_BUFFER);
    assert!(offset_of!(Spi, UniqueProcessId) == UNIQUE_PROCESS_ID);
    assert!(offset_of!(Spi, WorkingSetSize) == WORKING_SET_SIZE);
    assert!(offset_of!(Spi, PagefileUsage) == PAGEFILE_USAGE);
    // The I/O counters fill the tail that the header calls `Reserved7: [i64; 6]`.
    assert!(offset_of!(Spi, Reserved7) + 3 * 8 == READ_TRANSFER_COUNT);
};

/// The process list can grow between the size query and the copy, so a retry can fail
/// again. A few attempts are enough in practice; the bound keeps a pathological case
/// from spinning.
const MAX_ATTEMPTS: u32 = 4;

/// Error from taking or reading a process snapshot.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    /// `NtQuerySystemInformation` returned a failure status.
    #[error("NtQuerySystemInformation failed with NTSTATUS {0:#010x}")]
    Query(NTSTATUS),
    /// The snapshot would not fit in a buffer whose size fits in a `u32`.
    #[error("the process snapshot does not fit in a 4 GiB buffer")]
    TooLarge,
    /// The kernel returned no data, or an entry or the image name it points to lies
    /// outside the bytes the kernel wrote.
    #[error("the process snapshot is malformed at byte offset {0}")]
    Malformed(usize),
}

/// A reusable `NtQuerySystemInformation(SystemProcessInformation)` snapshot.
///
/// One snapshot costs about 6 ms of CPU on the reference machine, which is why the
/// daemon takes it every 2 seconds (ADR 0019). The buffer only grows when the process
/// list outgrows it, so steady-state refreshes do not allocate.
pub struct ProcessSnapshot {
    // u64 elements give the buffer the 8-byte alignment of the structures the kernel
    // writes into it.
    buffer: Vec<u64>,
    len: usize,
}

impl ProcessSnapshot {
    /// Creates an empty snapshot. It holds no processes until the first successful
    /// [`refresh`](Self::refresh).
    #[must_use]
    pub fn new() -> Self {
        // The reference machine needed about 1 MB with 390 processes and 8000 threads;
        // starting smaller costs one grow at start-up.
        const INITIAL_BYTES: usize = 256 * 1024;
        Self {
            buffer: vec![0; INITIAL_BYTES / 8],
            len: 0,
        }
    }

    /// Replaces the snapshot with the current process list.
    ///
    /// On failure the snapshot is left empty, so stale data is never read as current.
    pub fn refresh(&mut self) -> Result<(), SnapshotError> {
        self.len = 0;
        for _ in 0..MAX_ATTEMPTS {
            let capacity =
                u32::try_from(self.buffer.len() * 8).map_err(|_| SnapshotError::TooLarge)?;
            let mut needed = 0u32;
            // SAFETY: `buffer` is writable for `capacity` bytes and `needed` is a valid
            // out pointer. The kernel writes at most `capacity` bytes.
            let status = unsafe {
                NtQuerySystemInformation(
                    SystemProcessInformation,
                    self.buffer.as_mut_ptr().cast(),
                    capacity,
                    &mut needed,
                )
            };
            if status == STATUS_INFO_LENGTH_MISMATCH || status == STATUS_BUFFER_TOO_SMALL {
                // Headroom so that normal growth of the process list does not cost a
                // second call on the next refresh.
                let needed = needed.max(capacity) as usize;
                self.buffer.resize((needed + needed / 4).div_ceil(8), 0);
                continue;
            }
            if status < 0 {
                return Err(SnapshotError::Query(status));
            }
            if needed == 0 {
                // Every snapshot contains at least the idle process; an empty success
                // would otherwise read as a machine with no processes.
                return Err(SnapshotError::Malformed(0));
            }
            // Clamped so that `bytes` stays in bounds whatever length is reported.
            self.len = needed.min(capacity) as usize;
            return Ok(());
        }
        Err(SnapshotError::Query(STATUS_INFO_LENGTH_MISMATCH))
    }

    /// Iterates over the processes in the last successful refresh.
    pub fn processes(&self) -> Processes<'_> {
        Processes::new(self.bytes())
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the pointer comes from a live `Vec<u64>`, so it is valid and aligned
        // for `buffer.len() * 8` bytes, and `refresh` clamps `len` to that capacity.
        // `u8` has no invalid bit patterns, and the returned slice borrows `self`, so the
        // buffer cannot change while it is in use.
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), self.len) }
    }
}

impl Default for ProcessSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over the entries of a [`ProcessSnapshot`].
///
/// Yields an error and then stops if an entry lies outside the snapshot, so a caller
/// never mistakes a truncated list for a complete one.
pub struct Processes<'a> {
    bytes: &'a [u8],
    next: Option<usize>,
}

impl<'a> Processes<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            next: (!bytes.is_empty()).then_some(0),
        }
    }
}

impl<'a> Iterator for Processes<'a> {
    type Item = Result<ProcessRecord<'a>, SnapshotError>;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.next.take()?;
        match read_entry(self.bytes, offset) {
            Ok((record, next)) => {
                self.next = next;
                Some(Ok(record))
            }
            Err(error) => Some(Err(error)),
        }
    }
}

/// One process from a snapshot. Counters are cumulative since the process started.
#[derive(Debug, Clone, Copy)]
pub struct ProcessRecord<'a> {
    /// Process id. Ids are reused, so a process is identified by the id together with
    /// [`create_time`](Self::create_time).
    pub pid: u32,
    /// Creation time as a `FILETIME` value (100 ns units since 1601-01-01 UTC).
    pub create_time: u64,
    /// Short image name, such as `explorer.exe`. Empty for the idle process.
    pub image_name: ImageName<'a>,
    /// CPU cycles charged to the process's threads.
    pub cycle_time: u64,
    /// User-mode CPU time in 100 ns units, sampled at timer ticks.
    pub user_time: u64,
    /// Kernel-mode CPU time in 100 ns units, sampled at timer ticks.
    pub kernel_time: u64,
    /// Working set in bytes.
    pub working_set: u64,
    /// Private part of the working set in bytes (Task Manager's "Memory" column).
    pub private_working_set: u64,
    /// Private committed memory in bytes ("Private Bytes", "Commit size").
    pub private_bytes: u64,
    /// Bytes read through read operations, from files and devices alike.
    pub read_bytes: u64,
    /// Bytes written through write operations, to files and devices alike.
    pub write_bytes: u64,
    /// Bytes transferred by operations that are neither reads nor writes.
    pub other_bytes: u64,
}

/// A process image name as the kernel stores it: UTF-16 that borrows the snapshot.
///
/// Decoding allocates, so callers decode a name once, when they first see a process.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ImageName<'a>(&'a [u8]);

impl ImageName<'_> {
    /// Returns true for a process without a name (the idle process).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Decodes the name, replacing unpaired surrogates with U+FFFD.
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        let units = self
            .0
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
        char::decode_utf16(units)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }
}

impl fmt::Debug for ImageName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.to_string_lossy(), f)
    }
}

/// Reads the entry at `offset` and returns it with the offset of the next entry, if
/// there is one.
fn read_entry(
    bytes: &[u8],
    offset: usize,
) -> Result<(ProcessRecord<'_>, Option<usize>), SnapshotError> {
    let entry: &[u8; ENTRY_SIZE] = offset
        .checked_add(ENTRY_SIZE)
        .and_then(|end| bytes.get(offset..end))
        .and_then(|slice| slice.try_into().ok())
        .ok_or(SnapshotError::Malformed(offset))?;

    let next = match read_u32(entry, NEXT_ENTRY_OFFSET) {
        0 => None,
        // Each entry is followed by its thread records, so a valid step is at least one
        // entry long. A shorter step would revisit bytes or loop forever.
        step if (step as usize) < ENTRY_SIZE => return Err(SnapshotError::Malformed(offset)),
        step => Some(offset + step as usize),
    };

    let record = ProcessRecord {
        pid: u32::try_from(read_u64(entry, UNIQUE_PROCESS_ID))
            .map_err(|_| SnapshotError::Malformed(offset))?,
        create_time: read_u64(entry, CREATE_TIME),
        image_name: read_image_name(bytes, entry).ok_or(SnapshotError::Malformed(offset))?,
        cycle_time: read_u64(entry, CYCLE_TIME),
        user_time: read_u64(entry, USER_TIME),
        kernel_time: read_u64(entry, KERNEL_TIME),
        working_set: read_u64(entry, WORKING_SET_SIZE),
        private_working_set: read_u64(entry, WORKING_SET_PRIVATE_SIZE),
        private_bytes: read_u64(entry, PAGEFILE_USAGE),
        read_bytes: read_u64(entry, READ_TRANSFER_COUNT),
        write_bytes: read_u64(entry, WRITE_TRANSFER_COUNT),
        other_bytes: read_u64(entry, OTHER_TRANSFER_COUNT),
    };
    Ok((record, next))
}

/// Resolves the entry's `UNICODE_STRING`. The kernel stores an absolute pointer into the
/// same buffer; it is turned into an offset and bounds-checked instead of dereferenced.
fn read_image_name<'a>(bytes: &'a [u8], entry: &[u8; ENTRY_SIZE]) -> Option<ImageName<'a>> {
    let length = usize::from(read_u16(entry, IMAGE_NAME_LENGTH));
    let address = usize::try_from(read_u64(entry, IMAGE_NAME_BUFFER)).ok()?;
    if address == 0 {
        return (length == 0).then_some(ImageName(&[]));
    }
    if length % 2 != 0 {
        return None;
    }
    let start = address.checked_sub(bytes.as_ptr() as usize)?;
    let name = bytes.get(start..start.checked_add(length)?)?;
    Some(ImageName(name))
}

fn read_u16(entry: &[u8; ENTRY_SIZE], offset: usize) -> u16 {
    u16::from_le_bytes(field(entry, offset))
}

fn read_u32(entry: &[u8; ENTRY_SIZE], offset: usize) -> u32 {
    u32::from_le_bytes(field(entry, offset))
}

fn read_u64(entry: &[u8; ENTRY_SIZE], offset: usize) -> u64 {
    u64::from_le_bytes(field(entry, offset))
}

/// Copies `N` bytes at a fixed field offset. The offsets are the constants above, all
/// inside the entry, so the slice index cannot go out of bounds.
fn field<const N: usize>(entry: &[u8; ENTRY_SIZE], offset: usize) -> [u8; N] {
    let mut out = [0; N];
    out.copy_from_slice(&entry[offset..offset + N]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const THREAD_RECORD: usize = 80;
    const NAME_AREA: usize = 1024;

    /// Lays out a snapshot the way the kernel does: entries, each followed by its thread
    /// records, then the image names, referenced by absolute address. The parser does
    /// not depend on alignment, so a byte vector is enough.
    fn snapshot(entries: &[(u64, &str, usize)]) -> Vec<u8> {
        let steps: Vec<usize> = entries
            .iter()
            .map(|&(_, _, threads)| ENTRY_SIZE + threads * THREAD_RECORD)
            .collect();
        let entries_size: usize = steps.iter().sum();
        let mut bytes = vec![0; entries_size + NAME_AREA];
        let base = bytes.as_ptr() as u64;
        let mut offset = 0;
        let mut name_offset = entries_size;
        for (i, &(pid, name, _)) in entries.iter().enumerate() {
            let next = if i + 1 == entries.len() {
                0
            } else {
                steps[i] as u32
            };
            put(&mut bytes, offset + NEXT_ENTRY_OFFSET, &next.to_le_bytes());
            put(&mut bytes, offset + UNIQUE_PROCESS_ID, &pid.to_le_bytes());
            // A distinct value per field, so reading the wrong offset fails a test.
            let fields = [
                CREATE_TIME,
                CYCLE_TIME,
                USER_TIME,
                KERNEL_TIME,
                WORKING_SET_SIZE,
                WORKING_SET_PRIVATE_SIZE,
                PAGEFILE_USAGE,
                READ_TRANSFER_COUNT,
                WRITE_TRANSFER_COUNT,
                OTHER_TRANSFER_COUNT,
            ];
            for (tag, field) in (1u64..).zip(fields) {
                put(
                    &mut bytes,
                    offset + field,
                    &(tag * 1000 + pid).to_le_bytes(),
                );
            }
            if !name.is_empty() {
                let encoded: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
                let length = encoded.len() as u16;
                put(&mut bytes, name_offset, &encoded);
                put(
                    &mut bytes,
                    offset + IMAGE_NAME_LENGTH,
                    &length.to_le_bytes(),
                );
                let address = base + name_offset as u64;
                put(
                    &mut bytes,
                    offset + IMAGE_NAME_BUFFER,
                    &address.to_le_bytes(),
                );
                name_offset += encoded.len();
            }
            offset += steps[i];
        }
        bytes
    }

    fn put(bytes: &mut [u8], offset: usize, value: &[u8]) {
        bytes[offset..offset + value.len()].copy_from_slice(value);
    }

    fn parse(bytes: &[u8]) -> Result<Vec<ProcessRecord<'_>>, SnapshotError> {
        Processes::new(bytes).collect()
    }

    #[test]
    fn reads_every_entry_and_follows_the_chain() {
        let bytes = snapshot(&[(0, "", 2), (4, "System", 3), (1234, "explorer.exe", 0)]);
        let records = parse(&bytes).unwrap();

        let pids: Vec<u32> = records.iter().map(|r| r.pid).collect();
        assert_eq!(pids, [0, 4, 1234]);
        let names: Vec<String> = records
            .iter()
            .map(|r| r.image_name.to_string_lossy())
            .collect();
        assert_eq!(names, ["", "System", "explorer.exe"]);
        assert!(records[0].image_name.is_empty());

        let explorer = &records[2];
        assert_eq!(explorer.create_time, 2234);
        assert_eq!(explorer.cycle_time, 3234);
        assert_eq!(explorer.user_time, 4234);
        assert_eq!(explorer.kernel_time, 5234);
        assert_eq!(explorer.working_set, 6234);
        assert_eq!(explorer.private_working_set, 7234);
        assert_eq!(explorer.private_bytes, 8234);
        assert_eq!(explorer.read_bytes, 9234);
        assert_eq!(explorer.write_bytes, 10234);
        assert_eq!(explorer.other_bytes, 11234);
    }

    #[test]
    fn decodes_names_outside_ascii() {
        let bytes = snapshot(&[(8, "IŞIK-ığüşöç.exe", 1)]);
        let records = parse(&bytes).unwrap();
        assert_eq!(records[0].image_name.to_string_lossy(), "IŞIK-ığüşöç.exe");
    }

    #[test]
    fn empty_buffer_has_no_processes() {
        assert!(parse(&[]).unwrap().is_empty());
    }

    #[test]
    fn rejects_a_truncated_entry() {
        let bytes = snapshot(&[(4, "System", 0)]);
        let error = parse(&bytes[..ENTRY_SIZE - 1]).unwrap_err();
        assert!(matches!(error, SnapshotError::Malformed(0)));
    }

    #[test]
    fn reports_a_next_entry_beyond_the_buffer_then_stops() {
        let mut bytes = snapshot(&[(4, "System", 0)]);
        put(&mut bytes, NEXT_ENTRY_OFFSET, &65536u32.to_le_bytes());
        let mut processes = Processes::new(&bytes);

        assert!(processes.next().unwrap().is_ok());
        assert!(matches!(
            processes.next(),
            Some(Err(SnapshotError::Malformed(65536)))
        ));
        assert!(processes.next().is_none());
    }

    #[test]
    fn rejects_a_step_shorter_than_an_entry() {
        let mut bytes = snapshot(&[(4, "System", 0), (8, "a.exe", 0)]);
        put(&mut bytes, NEXT_ENTRY_OFFSET, &8u32.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(SnapshotError::Malformed(0))));
    }

    #[test]
    fn rejects_a_name_outside_the_buffer() {
        let mut bytes = snapshot(&[(4, "System", 0)]);
        put(&mut bytes, IMAGE_NAME_BUFFER, &16u64.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(SnapshotError::Malformed(0))));
    }

    #[test]
    fn rejects_a_name_with_an_odd_length() {
        let mut bytes = snapshot(&[(4, "System", 0)]);
        put(&mut bytes, IMAGE_NAME_LENGTH, &5u16.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(SnapshotError::Malformed(0))));
    }

    #[test]
    fn rejects_a_pid_wider_than_32_bits() {
        let mut bytes = snapshot(&[(4, "System", 0)]);
        let pid = u64::from(u32::MAX) + 1;
        put(&mut bytes, UNIQUE_PROCESS_ID, &pid.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(SnapshotError::Malformed(0))));
    }
}
