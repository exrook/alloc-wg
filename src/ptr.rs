use core::{
    convert::From,
    fmt,
    marker::{PhantomData, Unsize},
    ops::{CoerceUnsized, DispatchFromDyn},
    ptr::NonNull,
};

// ignore-tidy-undocumented-unsafe

/// A wrapper around a raw non-null `*mut T` that indicates that the possessor
/// of this wrapper owns the referent. Useful for building abstractions like
/// `Box<T>`, `Vec<T>`, `String`, and `HashMap<K, V>`.
///
/// Unlike `*mut T`, `Unique<T>` behaves "as if" it were an instance of `T`.
/// It implements `Send`/`Sync` if `T` is `Send`/`Sync`. It also implies
/// the kind of strong aliasing guarantees an instance of `T` can expect:
/// the referent of the pointer should not be modified without a unique path to
/// its owning Unique.
///
/// If you're uncertain of whether it's correct to use `Unique` for your purposes,
/// consider using `NonNull`, which has weaker semantics.
///
/// Unlike `*mut T`, the pointer must always be non-null, even if the pointer
/// is never dereferenced. This is so that enums may use this forbidden value
/// as a discriminant -- `Option<Unique<T>>` has the same size as `Unique<T>`.
/// However the pointer may still dangle if it isn't dereferenced.
///
/// Unlike `*mut T`, `Unique<T>` is covariant over `T`. This should always be correct
/// for any type which upholds Unique's aliasing requirements.
#[repr(transparent)]
pub struct Unique<T: ?Sized> {
    pointer: NonNull<T>,
    // NOTE: this marker has no consequences for variance, but is necessary
    // for dropck to understand that we logically own a `T`.
    //
    // For details, see:
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    _marker: PhantomData<T>,
}

impl<T: ?Sized> fmt::Debug for Unique<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.pointer.as_ptr(), fmt)
    }
}

/// `Unique` pointers are `Send` if `T` is `Send` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
unsafe impl<T: Send + ?Sized> Send for Unique<T> {}

/// `Unique` pointers are `Sync` if `T` is `Sync` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
unsafe impl<T: Sync + ?Sized> Sync for Unique<T> {}

impl<T: Sized> Unique<T> {
    /// Creates a new `Unique` that is dangling, but well-aligned.
    ///
    /// This is useful for initializing types which lazily allocate, like
    /// `Vec::new` does.
    ///
    /// Note that the pointer value may potentially represent a valid pointer to
    /// a `T`, which means this must not be used as a "not yet initialized"
    /// sentinel value. Types that lazily allocate must track initialization by
    /// some other means.
    #[inline]
    pub const fn dangling() -> Self {
        Self {
            pointer: NonNull::dangling(),
            _marker: PhantomData,
        }
    }
}

#[allow(clippy::use_self)]
impl<T: ?Sized> Unique<T> {
    /// Creates a new `Unique`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    #[inline]
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        Self {
            pointer: NonNull::new_unchecked(ptr),
            _marker: PhantomData,
        }
    }

    /// Creates a new `Unique` if `ptr` is non-null.
    #[inline]
    pub fn new(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(Into::into)
    }

    /// Acquires the underlying `*mut` pointer.
    #[inline]
    pub const fn as_ptr(self) -> *mut T {
        self.pointer.as_ptr()
    }

    /// Dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&*my_ptr.as_ptr()`.
    #[inline]
    pub unsafe fn as_ref(&self) -> &T {
        self.pointer.as_ref()
    }

    /// Mutably dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&mut *my_ptr.as_ptr()`.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        self.pointer.as_mut()
    }

    /// Casts to a pointer of another type.
    #[inline]
    pub const fn cast<U>(self) -> Unique<U> {
        Unique {
            pointer: self.pointer.cast(),
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized> Clone for Unique<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Unique<T> {}

impl<T: ?Sized, U: ?Sized> CoerceUnsized<Unique<U>> for Unique<T> where T: Unsize<U> {}

impl<T: ?Sized, U: ?Sized> DispatchFromDyn<Unique<U>> for Unique<T> where T: Unsize<U> {}

impl<T: ?Sized> From<NonNull<T>> for Unique<T> {
    #[inline]
    fn from(p: NonNull<T>) -> Self {
        unsafe { Self::new_unchecked(p.as_ptr()) }
    }
}

impl<T: ?Sized> From<Unique<T>> for NonNull<T> {
    #[inline]
    fn from(p: Unique<T>) -> Self {
        unsafe { Self::new_unchecked(p.as_ptr()) }
    }
}
