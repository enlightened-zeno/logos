/// Physical memory address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysAddr(u64);

/// Virtual memory address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(u64);

/// A 4 KiB-aligned physical frame number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysFrame(u64);

pub const PAGE_SIZE: u64 = 4096;
pub const PAGE_SHIFT: u64 = 12;

impl PhysAddr {
    /// Create a new physical address. Panics if bits 52..64 are set.
    #[inline]
    pub const fn new(addr: u64) -> Self {
        assert!(
            addr & 0xFFF0_0000_0000_0000 == 0,
            "PhysAddr: bits 52..64 must be zero"
        );
        Self(addr)
    }

    /// Create without validation — caller guarantees validity.
    #[inline]
    pub const fn new_unchecked(addr: u64) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn is_aligned(self, alignment: u64) -> bool {
        self.0 & (alignment - 1) == 0
    }

    #[inline]
    pub const fn is_page_aligned(self) -> bool {
        self.is_aligned(PAGE_SIZE)
    }

    #[inline]
    pub const fn align_up(self, alignment: u64) -> Self {
        Self((self.0 + alignment - 1) & !(alignment - 1))
    }

    #[inline]
    pub const fn align_down(self, alignment: u64) -> Self {
        Self(self.0 & !(alignment - 1))
    }

    /// Convert to the containing physical frame.
    #[inline]
    pub const fn containing_frame(self) -> PhysFrame {
        PhysFrame(self.0 >> PAGE_SHIFT)
    }
}

impl VirtAddr {
    /// Create a new canonical virtual address. Panics if not canonical.
    #[inline]
    pub const fn new(addr: u64) -> Self {
        // x86_64 canonical form: bits 48..64 must match bit 47
        let shifted = ((addr as i64) << 16) >> 16;
        assert!(shifted as u64 == addr, "VirtAddr: not canonical");
        Self(addr)
    }

    /// Create without validation.
    #[inline]
    pub const fn new_unchecked(addr: u64) -> Self {
        Self(addr)
    }

    /// Force an address into canonical form by sign-extending bit 47.
    #[inline]
    pub const fn new_canonicalize(addr: u64) -> Self {
        Self(((addr as i64) << 16 >> 16) as u64)
    }

    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    #[inline]
    pub const fn is_aligned(self, alignment: u64) -> bool {
        self.0 & (alignment - 1) == 0
    }

    #[inline]
    pub const fn align_up(self, alignment: u64) -> Self {
        Self::new_canonicalize((self.0 + alignment - 1) & !(alignment - 1))
    }

    #[inline]
    pub const fn align_down(self, alignment: u64) -> Self {
        Self::new_canonicalize(self.0 & !(alignment - 1))
    }

    /// Page table indices for 4-level paging.
    #[inline]
    pub const fn p4_index(self) -> usize {
        ((self.0 >> 39) & 0x1FF) as usize
    }

    #[inline]
    pub const fn p3_index(self) -> usize {
        ((self.0 >> 30) & 0x1FF) as usize
    }

    #[inline]
    pub const fn p2_index(self) -> usize {
        ((self.0 >> 21) & 0x1FF) as usize
    }

    #[inline]
    pub const fn p1_index(self) -> usize {
        ((self.0 >> 12) & 0x1FF) as usize
    }

    #[inline]
    pub const fn page_offset(self) -> u64 {
        self.0 & 0xFFF
    }

    #[inline]
    pub const fn offset(self, offset: u64) -> Self {
        Self::new_canonicalize(self.0.wrapping_add(offset))
    }
}

impl PhysFrame {
    /// Create from frame number.
    #[inline]
    pub const fn from_number(n: u64) -> Self {
        Self(n)
    }

    /// Create from the physical address at the start of the frame.
    #[inline]
    pub const fn containing_address(addr: PhysAddr) -> Self {
        Self(addr.as_u64() >> PAGE_SHIFT)
    }

    #[inline]
    pub const fn number(self) -> u64 {
        self.0
    }

    /// Start address of this frame.
    #[inline]
    pub const fn start_address(self) -> PhysAddr {
        PhysAddr::new_unchecked(self.0 << PAGE_SHIFT)
    }

    #[inline]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

// Arithmetic ops

impl core::ops::Add<u64> for PhysAddr {
    type Output = PhysAddr;
    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        PhysAddr::new(self.0 + rhs)
    }
}

impl core::ops::Sub<PhysAddr> for PhysAddr {
    type Output = u64;
    #[inline]
    fn sub(self, rhs: PhysAddr) -> u64 {
        self.0 - rhs.0
    }
}

impl core::ops::Add<u64> for VirtAddr {
    type Output = VirtAddr;
    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        VirtAddr::new_canonicalize(self.0 + rhs)
    }
}

impl core::ops::Sub<VirtAddr> for VirtAddr {
    type Output = u64;
    #[inline]
    fn sub(self, rhs: VirtAddr) -> u64 {
        self.0.wrapping_sub(rhs.0)
    }
}

// Debug/Display impls

impl core::fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PhysAddr({:#x})", self.0)
    }
}

impl core::fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl core::fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VirtAddr({:#x})", self.0)
    }
}

impl core::fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl core::fmt::Debug for PhysFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PhysFrame(#{}, addr={:#x})",
            self.0,
            self.0 << PAGE_SHIFT
        )
    }
}
