#![no_std]

//! The `array-init` crate allows you to initialize arrays
//! with an initializer closure that will be called
//! once for each element until the array is filled.
//!
//! This way you do not need to default-fill an array
//! before running initializers. Rust currently only
//! lets you either specify all initializers at once,
//! individually (`[a(), b(), c(), ...]`), or specify
//! one initializer for a `Copy` type (`[a(); N]`),
//! which will be called once with the result copied over.
//!
//! # Examples:
//! ```rust
//! # #![allow(unused)]
//! # extern crate array_init;
//! #
//! // Initialize an array of length 50 containing
//! // successive squares
//!
//! let arr: [u32; 50] = array_init::array_init(|i: usize| (i * i) as u32);
//!
//! // Initialize an array from an iterator
//! // producing an array of [1,2,3,4] repeated
//!
//! let four = [1,2,3,4];
//! let mut iter = four.iter().copied().cycle();
//! let arr: [u32; 50] = array_init::from_iter(iter).unwrap();
//!
//! // Closures can also mutate state. We guarantee that they will be called
//! // in order from lower to higher indices.
//!
//! let mut last = 1u64;
//! let mut secondlast = 0;
//! let fibonacci: [u64; 50] = array_init::array_init(|_| {
//!     let this = last + secondlast;
//!     secondlast = last;
//!     last = this;
//!     this
//! });
//! ```

use ::core::{
    mem::{self,
        MaybeUninit,
    },
    ptr,
    slice,
};

/// Trait for things which are actually arrays.
///
/// Probably shouldn't implement this yourself, but you can.
///
/// # Safety
///
///   - if `Array : IsArray`, then it must be sound to transmute
///     between `Array` and `[Array::Item; Array::len()]`
pub unsafe trait IsArray {
    /// The stored `T`
    type Item;

    /// The number of elements of the array
    fn len() -> usize;
}

#[inline]
/// Initialize an array given an initializer expression.
///
/// The initializer is given the index of the element. It is allowed
/// to mutate external state; we will always initialize the elements in order.
///
/// # Examples
///
/// ```rust
/// # #![allow(unused)]
/// # extern crate array_init;
/// #
/// // Initialize an array of length 50 containing
/// // successive squares
/// let arr: [usize; 50] = array_init::array_init(|i| i * i);
///
/// assert!(arr.iter().enumerate().all(|(i, &x)| x == i * i));
/// ```
pub fn array_init<Array, F> (
    mut initializer: F,
) -> Array
where
    Array : IsArray,
    F : FnMut(usize) -> Array::Item,
{
    enum Unreachable {}

    try_array_init( // monomorphise into an unfallible version
        move |i| -> Result<Array::Item, Unreachable> {
            Ok(initializer(i))
        }
    ).unwrap_or_else( // zero-cost unwrap
        |unreachable| match unreachable { /* ! */ }
    )
}

#[inline]
/// Initialize an array given an iterator
///
/// We will iterate until the array is full or the iterator is exhausted. Returns
/// `None` if the iterator is exhausted before we can fill the array.
///
///   - Once the array is full, extra elements from the iterator (if any)
///     won't be consumed.
///
/// # Examples
///
/// ```rust
/// # #![allow(unused)]
/// # extern crate array_init;
/// #
/// // Initialize an array from an iterator
/// // producing an array of [1,2,3,4] repeated
///
/// let four = [1,2,3,4];
/// let mut iter = four.iter().copied().cycle();
/// let arr: [u32; 50] = array_init::from_iter(iter).unwrap();
/// ```
pub fn from_iter<Array, Iterable> (
    iterable: Iterable,
) -> Option<Array>
where
    Iterable : IntoIterator<Item = Array::Item>,
    Array : IsArray,
{
    try_array_init({
        let mut iterator = iterable.into_iter();
        move |_| {
            iterator.next().ok_or(())
        }
    }).ok()
}

#[inline]
pub fn try_array_init<Array, Err, F> (
    mut initializer: F,
) -> Result<Array, Err>
where
    Array : IsArray,
    F : FnMut(usize) -> Result<Array::Item, Err>,
{
    if !mem::needs_drop::<Array::Item>() {
        let mut array: MaybeUninit<Array> = MaybeUninit::uninit();
        // pointer to array = *mut [T; N] <-> *mut T = pointer to first element
        let mut ptr_i = array.as_mut_ptr() as *mut Array::Item;

        //   - Using `ptr::add` instead of `offset` avoids having to check
        //     that the offset in bytes does not overflow isize.
        //
        // # Safety
        //
        //   - `IsArray`'s contract guarantees that we are within the array
        //     since we have `0 <= i < Array::len`
        unsafe {
            for i in 0 .. Array::len() {
                let value_i = initializer(i)?;
                ptr_i.write(value_i);
                ptr_i = ptr_i.add(1);
            }
            return Ok(array.assume_init());
        }
    }

    // else: `mem::needs_drop::<Array::Item>()`

    /// # Safety
    ///
    ///   - `base_ptr[.. initialized_count]` must be a slice of init elements...
    ///
    ///   - ... that must be sound to `ptr::drop_in_place` if/when
    ///     `UnsafeDropSliceGuard` is dropped: "symbolic ownership"
    struct UnsafeDropSliceGuard<Item> {
        base_ptr: *mut Item,
        initialized_count: usize,
    }

    impl<Item> Drop for UnsafeDropSliceGuard<Item> {
        fn drop (self: &'_ mut Self)
        {
            unsafe {
                // # Safety
                //
                //   - the contract of the struct guarantees that this is sound
                ptr::drop_in_place(
                    slice::from_raw_parts_mut(
                        self.base_ptr,
                        self.initialized_count,
                    )
                );
            }
        }
    }

    //  1. If the `initializer(i)` call panics, `panic_guard` is dropped,
    //     dropping `array[.. initialized_count]` => no memory leak!
    //
    //  2. Using `ptr::add` instead of `offset` avoids having to check
    //     that the offset in bytes does not overflow isize.
    //
    // # Safety
    //
    //  1. By construction, array[.. initiliazed_count] only contains
    //     init elements, thus there is no risk of dropping uninit data;
    //
    //  2. `IsArray`'s contract guarantees that we are within the array
    //     since we have `0 <= i < Array::len`
    unsafe {
        let mut array: MaybeUninit<Array> = MaybeUninit::uninit();
        // pointer to array = *mut [T; N] <-> *mut T = pointer to first element
        let mut ptr_i = array.as_mut_ptr() as *mut Array::Item;
        let mut panic_guard = UnsafeDropSliceGuard {
            base_ptr: ptr_i,
            initialized_count: 0,
        };

        for i in 0 .. Array::len() {
            // Invariant: `i` elements have already been initialized
            panic_guard.initialized_count = i;
            // If this panics or fails, `panic_guard` is dropped, thus
            // dropping the elements in `base_ptr[.. i]`
            let value_i = initializer(i)?;
            // this cannot panic
            ptr_i.write(value_i);
            ptr_i = ptr_i.add(1);
        }
        // From now on, the code can no longer `panic!`, let's take the
        // symbolic ownership back
        mem::forget(panic_guard);

        Ok(array.assume_init())
    }
}

macro_rules! impl_is_array {
    ($($size:expr)+) => ($(
        unsafe impl<T> IsArray for [T; $size] {
            type Item = T;

            #[inline]
            fn len() -> usize {
                $size
            }
        }
    )+)
}

// lol

impl_is_array! {
     0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15
    16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
    32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47
    48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63
    64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79
    80 81 82 83 84 85 86 87 88 89 90 91 92 93 94 95
    96 97 98 99 100 101 102 103 104 105 106 107 108
    109 110 111 112 113 114 115 116 117 118 119 120
    121 122 123 124 125 126 127 128 129 130 131 132
    133 134 135 136 137 138 139 140 141 142 143 144
    145 146 147 148 149 150 151 152 153 154 155 156
    157 158 159 160 161 162 163 164 165 166 167 168
    169 170 171 172 173 174 175 176 177 178 179 180
    181 182 183 184 185 186 187 188 189 190 191 192
    193 194 195 196 197 198 199 200 201 202 203 204
    205 206 207 208 209 210 211 212 213 214 215 216
    217 218 219 220 221 222 223 224 225 226 227 228
    229 230 231 232 233 234 235 236 237 238 239 240
    241 242 243 244 245 246 247 248 249 250 251 252
    253 254 255 256 257 258 259 260 261 262 263 264
    265 266 267 268 269 270 271 272 273 274 275 276
    277 278 279 280 281 282 283 284 285 286 287 288
    289 290 291 292 293 294 295 296 297 298 299 300
    301 302 303 304 305 306 307 308 309 310 311 312
    313 314 315 316 317 318 319 320 321 322 323 324
    325 326 327 328 329 330 331 332 333 334 335 336
    337 338 339 340 341 342 343 344 345 346 347 348
    349 350 351 352 353 354 355 356 357 358 359 360
    361 362 363 364 365 366 367 368 369 370 371 372
    373 374 375 376 377 378 379 380 381 382 383 384
    385 386 387 388 389 390 391 392 393 394 395 396
    397 398 399 400 401 402 403 404 405 406 407 408
    409 410 411 412 413 414 415 416 417 418 419 420
    421 422 423 424 425 426 427 428 429 430 431 432
    433 434 435 436 437 438 439 440 441 442 443 444
    445 446 447 448 449 450 451 452 453 454 455 456
    457 458 459 460 461 462 463 464 465 466 467 468
    469 470 471 472 473 474 475 476 477 478 479 480
    481 482 483 484 485 486 487 488 489 490 491 492
    493 494 495 496 497 498 499 500 501 502 503 504
    505 506 507 508 509 510 511 512
    513   514   515   516   517   518   519   520   521   522   523   524   525
  526   527   528   529   530   531   532   533   534   535   536   537   538   539   540
  541   542   543   544   545   546   547   548   549   550   551   552   553   554   555
  556   557   558   559   560   561   562   563   564   565   566   567   568   569   570
  571   572   573   574   575   576   577   578   579   580   581   582   583   584   585
  586   587   588   589   590   591   592   593   594   595   596   597   598   599   600
  601   602   603   604   605   606   607   608   609   610   611   612   613   614   615
  616   617   618   619   620   621   622   623   624   625   626   627   628   629   630
  631   632   633   634   635   636   637   638   639   640   641   642   643   644   645
  646   647   648   649   650   651   652   653   654   655   656   657   658   659   660
  661   662   663   664   665   666   667   668   669   670   671   672   673   674   675
  676   677   678   679   680   681   682   683   684   685   686   687   688   689   690
  691   692   693   694   695   696   697   698   699   700   701   702   703   704   705
  706   707   708   709   710   711   712   713   714   715   716   717   718   719   720
  721   722   723   724   725   726   727   728   729   730   731   732   733   734   735
  736   737   738   739   740   741   742   743   744   745   746   747   748   749   750
  751   752   753   754   755   756   757   758   759   760   761   762   763   764   765
  766   767   768   769   770   771   772   773   774   775   776   777   778   779   780
  781   782   783   784   785   786   787   788   789   790   791   792   793   794   795
  796   797   798   799   800   801   802   803   804   805   806   807   808   809   810
  811   812   813   814   815   816   817   818   819   820   821   822   823   824   825
  826   827   828   829   830   831   832   833   834   835   836   837   838   839   840
  841   842   843   844   845   846   847   848   849   850   851   852   853   854   855
  856   857   858   859   860   861   862   863   864   865   866   867   868   869   870
  871   872   873   874   875   876   877   878   879   880   881   882   883   884   885
  886   887   888   889   890   891   892   893   894   895   896   897   898   899   900
  901   902   903   904   905   906   907   908   909   910   911   912   913   914   915
  916   917   918   919   920   921   922   923   924   925   926   927   928   929   930
  931   932   933   934   935   936   937   938   939   940   941   942   943   944   945
  946   947   948   949   950   951   952   953   954   955   956   957   958   959   960
  961   962   963   964   965   966   967   968   969   970   971   972   973   974   975
  976   977   978   979   980   981   982   983   984   985   986   987   988   989   990
  991   992   993   994   995   996   997   998   999   1000   1001   1002   1003   1004   1005
  1006   1007   1008   1009   1010   1011   1012   1013   1014   1015   1016   1017   1018   1019   1020
  1021   1022   1023   1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq ()
    {
        let seq: [usize; 5] = array_init(|i| i);
        assert_eq!(&[0, 1, 2, 3, 4], &seq);
    }

    #[test]
    fn array_from_iter ()
    {
        let array = [0, 1, 2, 3, 4];
        let seq: [usize; 5] = from_iter(array.iter().copied()).unwrap();
        assert_eq!(
            array,
            seq,
        );
    }

    #[test]
    fn array_init_no_drop ()
    {
        DropChecker::with(|drop_checker| {
            let result: Result<[_; 5], ()> =
                try_array_init(|i| {
                    if i < 3 {
                        Ok(drop_checker.new_element())
                    } else {
                        Err(())
                    }
                })
            ;
            assert!(result.is_err());
        });
    }

    #[test]
    fn from_iter_no_drop ()
    {
        DropChecker::with(|drop_checker| {
            let iterator = (0 .. 3).map(|_| drop_checker.new_element());
            let result: Option<[_; 5]> = from_iter(iterator);
            assert!(result.is_none());
        });
    }

    use self::drop_checker::DropChecker;
    mod drop_checker {
        use ::core::cell::Cell;

        pub(in super)
        struct DropChecker {
            slots: [Cell<bool>; 512],
            next_uninit_slot: Cell<usize>,
        }

        pub(in super)
        struct Element<'drop_checker> {
            slot: &'drop_checker Cell<bool>,
        }

        impl Drop for Element<'_> {
            fn drop (self: &'_ mut Self)
            {
                assert!(self.slot.replace(false), "Double free!");
            }
        }

        impl DropChecker {
            pub(in super)
            fn with (f: impl FnOnce(&Self))
            {
                let drop_checker = Self::new();
                f(&drop_checker);
                drop_checker.assert_no_leaks();
            }

            pub(in super)
            fn new_element (self: &'_ Self) -> Element<'_>
            {
                let i = self.next_uninit_slot.get();
                self.next_uninit_slot.set(i + 1);
                self.slots[i].set(true);
                Element {
                    slot: &self.slots[i],
                }
            }

            fn new () -> Self
            {
                Self {
                    slots: crate::array_init(|_| Cell::new(false)),
                    next_uninit_slot: Cell::new(0),
                }
            }

            fn assert_no_leaks (self: Self)
            {
                let leak_count: usize =
                    self.slots[.. self.next_uninit_slot.get()]
                        .iter()
                        .map(|slot| usize::from(slot.get() as u8))
                        .sum()
                ;
                assert_eq!(leak_count, 0, "No elements leaked");
            }
        }
    }
}
