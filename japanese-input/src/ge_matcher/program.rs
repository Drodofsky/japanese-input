// @generated — do not edit by hand. 504 nodes, 0 helper functions; 42/42 curated, 3000/3306 = 90.7% held out.

use super::runtime::Runtime;
use super::runtime::to_index;

/// Builds the whole assignment for one drawing, leaving the result in `rt`.
pub fn construct<R: Runtime>(rt: &mut R) {
    let mut r0: usize = 0;
    let mut r1: Option<usize> = Some(0);
    let mut r2: usize = 0;
    let mut r3: Option<usize> = Some(0);
    let mut r4: f64 = 0.0;
    let mut r5: f64 = 0.0;
    let mut r6: f64 = 0.0;
    let mut r7: f64 = 0.0;
    let mut r12: usize = 0;
    let mut r13: usize = 0;
    let mut r14: f64 = 0.0;
    let mut r15: Option<usize> = Some(0);
    let mut r16: usize = 0;
    let mut r17: usize = 0;
    let mut r18: usize = 0;
    let mut r19: usize = 0;
    let mut r20: usize = 0;
    let mut r21: f64 = 0.0;
    let mut r22: usize = 0;
    let mut r23: f64 = 0.0;
    let mut r24: f64 = 0.0;
    let mut r25: f64 = 0.0;
    let mut r26: f64 = 0.0;
    let mut r27: f64 = 0.0;
    let mut r28: usize = 0;
    let mut r29: f64 = 0.0;
    let mut r30: f64 = 0.0;
    let mut r31: f64 = 0.0;
    let _ = {
        let _ = {
            let mut iterations = 0_u32;
            loop {
                if !({ let left = rt.open_stroke_count(); let right = rt.open_leaf_count(); left < right }) { break; }
                let _ = {
                    rt.pop_state();
                    let _ = {
                        r24 = 10000000.0_f64;
                        r25 = -1.0_f64;
                        r26 = -1.0_f64;
                        r27 = -1.0_f64;
                        ({
                            let total = rt.leaf_count();
                            for i in 0..total {
                                r20 = i;
                                let _ = {
                                    let condition = { let a0 = Some(r20); rt.is_leaf_open(a0) };
                                    if condition {
                                        {
                                        let _ = {
                                            let condition = {
                                                let a0 = Some({
                                                    let left = r20;
                                                    let right = 1_usize;
                                                    left.saturating_add(right)
                                                });
                                                rt.is_leaf_open(a0)
                                            };
                                            if condition {
                                                {
                                                let _ = {
                                                    r31 = {
                                                        let left = { let a0 = Some(r20); rt.best_match_for_leaf(a0) };
                                                        let right = 0.0_f64;
                                                        left + right
                                                    };
                                                    {
                                                        let value = {
                                                            let left = r31;
                                                            let right = {
                                                                let a0 = Some({
                                                                    let left = r20;
                                                                    let right = 1_usize;
                                                                    left.saturating_add(right)
                                                                });
                                                                rt.best_match_for_leaf(a0)
                                                            };
                                                            left + right
                                                        };
                                                        r31 = value;
                                                        value
                                                    }
                                                };
                                                ({
                                                    let total = rt.stroke_count();
                                                    for i in 0..total {
                                                        r22 = i;
                                                        let _ = {
                                                            let condition = { let a0 = Some(r22); rt.is_stroke_open(a0) };
                                                            if condition {
                                                                {
                                                                r23 = {
                                                                    let a0 = Some(r20);
                                                                    let a1 = Some(2_usize);
                                                                    let a2 = Some(r22);
                                                                    rt.merge_shape_cost(a0, a1, a2)
                                                                };
                                                                {
                                                                    let condition = {
                                                                        let a0 = {
                                                                            let left = r23;
                                                                            let right = r24;
                                                                            left < right
                                                                        };
                                                                        let a1 = {
                                                                            let left = r23;
                                                                            let right = r31;
                                                                            left < right
                                                                        };
                                                                        a0 && a1
                                                                    };
                                                                    if condition {
                                                                        {
                                                                        r24 = r23;
                                                                        r25 = r20 as f64;
                                                                        r26 = 2.0_f64;
                                                                        {
                                                                            let value = r22 as f64;
                                                                            r27 = value;
                                                                            value
                                                                        }
                                                                    }
                                                                    } else {
                                                                        0.0
                                                                    }
                                                                }
                                                            }
                                                            } else {
                                                                0.0
                                                            }
                                                        };
                                                    }
                                                    0_usize
                                                }) as f64
                                            }
                                            } else {
                                                0.0
                                            }
                                        };
                                        {
                                            let condition = {
                                                let a0 = {
                                                    let a0 = Some({
                                                        let left = r20;
                                                        let right = 1_usize;
                                                        left.saturating_add(right)
                                                    });
                                                    rt.is_leaf_open(a0)
                                                };
                                                let a1 = {
                                                    let a0 = Some({
                                                        let left = r20;
                                                        let right = 2_usize;
                                                        left.saturating_add(right)
                                                    });
                                                    rt.is_leaf_open(a0)
                                                };
                                                a0 && a1
                                            };
                                            if condition {
                                                {
                                                let _ = {
                                                    r31 = {
                                                        let left = { let a0 = Some(r20); rt.best_match_for_leaf(a0) };
                                                        let right = 0.0_f64;
                                                        left + right
                                                    };
                                                    r31 = {
                                                        let left = r31;
                                                        let right = {
                                                            let a0 = Some({
                                                                let left = r20;
                                                                let right = 1_usize;
                                                                left.saturating_add(right)
                                                            });
                                                            rt.best_match_for_leaf(a0)
                                                        };
                                                        left + right
                                                    };
                                                    {
                                                        let value = {
                                                            let left = r31;
                                                            let right = {
                                                                let a0 = Some({
                                                                    let left = r20;
                                                                    let right = 2_usize;
                                                                    left.saturating_add(right)
                                                                });
                                                                rt.best_match_for_leaf(a0)
                                                            };
                                                            left + right
                                                        };
                                                        r31 = value;
                                                        value
                                                    }
                                                };
                                                ({
                                                    let total = rt.stroke_count();
                                                    for i in 0..total {
                                                        r22 = i;
                                                        let _ = {
                                                            let condition = { let a0 = Some(r22); rt.is_stroke_open(a0) };
                                                            if condition {
                                                                {
                                                                r23 = {
                                                                    let a0 = Some(r20);
                                                                    let a1 = Some(3_usize);
                                                                    let a2 = Some(r22);
                                                                    rt.merge_shape_cost(a0, a1, a2)
                                                                };
                                                                {
                                                                    let condition = {
                                                                        let a0 = {
                                                                            let left = r23;
                                                                            let right = r24;
                                                                            left < right
                                                                        };
                                                                        let a1 = {
                                                                            let left = r23;
                                                                            let right = r31;
                                                                            left < right
                                                                        };
                                                                        a0 && a1
                                                                    };
                                                                    if condition {
                                                                        {
                                                                        r24 = r23;
                                                                        r25 = r20 as f64;
                                                                        r26 = 3.0_f64;
                                                                        {
                                                                            let value = r22 as f64;
                                                                            r27 = value;
                                                                            value
                                                                        }
                                                                    }
                                                                    } else {
                                                                        0.0
                                                                    }
                                                                }
                                                            }
                                                            } else {
                                                                0.0
                                                            }
                                                        };
                                                    }
                                                    0_usize
                                                }) as f64
                                            }
                                            } else {
                                                0.0
                                            }
                                        }
                                    }
                                    } else {
                                        0.0
                                    }
                                };
                            }
                            0_usize
                        }) as f64
                    };
                    {
                        let condition = {
                            let a0 = { let left = r25; let right = -1.0_f64; (left - right).abs() < 1e-6 };
                            !a0
                        };
                        if condition {
                            ({
                            let a0 = to_index(r25);
                            let a1 = to_index(r26);
                            let a2 = to_index(r27);
                            rt.commit_merge(a0, a1, a2);
                            0_usize
                        }) as f64
                        } else {
                            {
                            r21 = -1000000.0_f64;
                            r30 = -1.0_f64;
                            let _ = {
                                let total = rt.leaf_count();
                                for i in 0..total {
                                    r28 = i;
                                    let _ = {
                                        let condition = { let a0 = Some(r28); rt.is_leaf_open(a0) };
                                        if condition {
                                            {
                                            r29 = { let a0 = Some(r28); rt.best_match_for_leaf(a0) };
                                            {
                                                let condition = { let left = r21; let right = r29; left < right };
                                                if condition {
                                                    { r21 = r29; { let value = r28 as f64; r30 = value; value } }
                                                } else {
                                                    0.0
                                                }
                                            }
                                        }
                                        } else {
                                            0.0
                                        }
                                    };
                                }
                                0_usize
                            };
                            ({ let a0 = to_index(r30); rt.commit_missing(a0); 0_usize }) as f64
                        }
                        }
                    }
                };
                iterations += 1;
                if iterations >= 128 { break; }
            }
            0_usize
        };
        let _ = {
            let mut iterations = 0_u32;
            loop {
                if !({
                    let a0 = { let left = 0_usize; let right = rt.open_leaf_count(); left < right };
                    let a1 = { let left = 0_usize; let right = rt.open_stroke_count(); left < right };
                    a0 && a1
                }) { break; }
                let _ = {
                    r4 = 10000000.0_f64;
                    r5 = -1.0_f64;
                    r6 = -1.0_f64;
                    r0 = 0_usize;
                    let _ = {
                        let mut iterations = 0_u32;
                        loop {
                            if !({
                                let a0 = {
                                    let left = { let index = { let a0 = Some(r0); rt.nth_open_leaf(a0) }; match index { Some(index) => index as f64, None => -1.0 } };
                                    let right = 0.0_f64;
                                    left < right
                                };
                                !a0
                            }) { break; }
                            let _ = {
                                r1 = { let a0 = Some(r0); rt.nth_open_leaf(a0) };
                                r2 = 0_usize;
                                let _ = {
                                    let mut iterations = 0_u32;
                                    loop {
                                        if !({
                                            let a0 = {
                                                let left = { let index = { let a0 = Some(r2); rt.nth_open_stroke(a0) }; match index { Some(index) => index as f64, None => -1.0 } };
                                                let right = 0.0_f64;
                                                left < right
                                            };
                                            !a0
                                        }) { break; }
                                        let _ = {
                                            r3 = { let a0 = Some(r2); rt.nth_open_stroke(a0) };
                                            r7 = { let a0 = r1; let a1 = r3; rt.shape_cost(a0, a1) };
                                            let _ = {
                                                let condition = { let left = r7; let right = r4; left < right };
                                                if condition {
                                                    {
                                                    r4 = r7;
                                                    r5 = match r1 { Some(index) => index as f64, None => -1.0 };
                                                    {
                                                        let value = match r3 { Some(index) => index as f64, None => -1.0 };
                                                        r6 = value;
                                                        value
                                                    }
                                                }
                                                } else {
                                                    0.0
                                                }
                                            };
                                            ({
                                                let value = {
                                                    let left = r2;
                                                    let right = 1_usize;
                                                    left.saturating_add(right)
                                                };
                                                r2 = value;
                                                value
                                            }) as f64
                                        };
                                        iterations += 1;
                                        if iterations >= 128 { break; }
                                    }
                                    0_usize
                                };
                                ({
                                    let value = { let left = r0; let right = 1_usize; left.saturating_add(right) };
                                    r0 = value;
                                    value
                                }) as f64
                            };
                            iterations += 1;
                            if iterations >= 128 { break; }
                        }
                        0_usize
                    };
                    ({ let a0 = to_index(r5); let a1 = to_index(r6); rt.commit_match(a0, a1); 0_usize }) as f64
                };
                iterations += 1;
                if iterations >= 128 { break; }
            }
            0_usize
        };
        let _ = r22;
        let _ = {
            let total = rt.leaf_count();
            for i in 0..total {
                r12 = i;
                let _ = {
                    let total = rt.leaf_count();
                    for i in 0..total {
                        r13 = i;
                        let _ = {
                            let condition = { let left = r12; let right = r13; left < right };
                            if condition {
                                {
                                r14 = {
                                    let a0 = 1_usize;
                                    let a1 = Some(r12);
                                    let a2 = Some(r13);
                                    rt.swap_gain(a0, a1, a2)
                                };
                                {
                                    let condition = { let left = r14; let right = 0.0_f64; left < right };
                                    if condition {
                                        ({
                                        let a0 = Some(r12);
                                        let a1 = Some(r13);
                                        rt.swap_matches(a0, a1);
                                        0_usize
                                    }) as f64
                                    } else {
                                        0.0
                                    }
                                }
                            }
                            } else {
                                0.0
                            }
                        };
                    }
                    0_usize
                };
            }
            0_usize
        };
        let _ = {
            let condition = { let left = r7; let right = r4; left < right };
            if condition {
                {
                r4 = r7;
                r5 = match r1 { Some(index) => index as f64, None => -1.0 };
                { let value = match r3 { Some(index) => index as f64, None => -1.0 }; r6 = value; value }
            }
            } else {
                0.0
            }
        };
        let _ = {
            let total = rt.leaf_count();
            for i in 0..total {
                r18 = i;
                let _ = {
                    let total = rt.leaf_count();
                    for i in 0..total {
                        r19 = i;
                        let _ = {
                            let condition = { let left = r18; let right = r19; left < right };
                            if condition {
                                {
                                {
                                    let a0 = {
                                        let left = {
                                            let left = {
                                                let left = {
                                                    let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                                    let right = {
                                                        let left = 0.5_f64;
                                                        let right = { let a0 = 0_usize; rt.total_leaf_cost(a0) };
                                                        left * right
                                                    };
                                                    left + right
                                                };
                                                let right = {
                                                    let left = 4.0_f64;
                                                    let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                                    left * right
                                                };
                                                left + right
                                            };
                                            let right = {
                                                let left = 4.0_f64;
                                                let right = { let a0 = 2_usize; rt.total_leaf_cost(a0) };
                                                left * right
                                            };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.push_state(a0);
                                }
                                { let a0 = Some(r18); let a1 = Some(r19); rt.swap_matches(a0, a1); }
                                ({
                                    let a0 = {
                                        let left = {
                                            let left = {
                                                let left = {
                                                    let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                                    let right = {
                                                        let left = 0.5_f64;
                                                        let right = { let a0 = 0_usize; rt.total_leaf_cost(a0) };
                                                        left * right
                                                    };
                                                    left + right
                                                };
                                                let right = {
                                                    let left = 4.0_f64;
                                                    let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                                    left * right
                                                };
                                                left + right
                                            };
                                            let right = {
                                                let left = 4.0_f64;
                                                let right = { let a0 = 2_usize; rt.total_leaf_cost(a0) };
                                                left * right
                                            };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.keep_best_state(a0);
                                    0_usize
                                }) as f64
                            }
                            } else {
                                0.0
                            }
                        };
                    }
                    0_usize
                };
            }
            0_usize
        };
        let _ = {
            let total = rt.leaf_count();
            for i in 0..total {
                r18 = i;
                let _ = {
                    let total = rt.leaf_count();
                    for i in 0..total {
                        r19 = i;
                        let _ = {
                            let condition = { let left = r18; let right = r19; left < right };
                            if condition {
                                {
                                {
                                    let a0 = {
                                        let left = {
                                            let left = {
                                                let left = {
                                                    let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                                    let right = {
                                                        let left = 0.5_f64;
                                                        let right = { let a0 = 0_usize; rt.total_leaf_cost(a0) };
                                                        left * right
                                                    };
                                                    left + right
                                                };
                                                let right = {
                                                    let left = 4.0_f64;
                                                    let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                                    left * right
                                                };
                                                left + right
                                            };
                                            let right = {
                                                let left = 4.0_f64;
                                                let right = { let a0 = 2_usize; rt.total_leaf_cost(a0) };
                                                left * right
                                            };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.push_state(a0);
                                }
                                { let a0 = Some(r18); let a1 = Some(r19); rt.swap_matches(a0, a1); }
                                ({
                                    let a0 = {
                                        let left = {
                                            let left = {
                                                let left = {
                                                    let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                                    let right = {
                                                        let left = 0.5_f64;
                                                        let right = { let a0 = 0_usize; rt.total_leaf_cost(a0) };
                                                        left * right
                                                    };
                                                    left + right
                                                };
                                                let right = {
                                                    let left = 4.0_f64;
                                                    let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                                    left * right
                                                };
                                                left + right
                                            };
                                            let right = {
                                                let left = 4.0_f64;
                                                let right = { let a0 = 2_usize; rt.total_leaf_cost(a0) };
                                                left * right
                                            };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.keep_best_state(a0);
                                    0_usize
                                }) as f64
                            }
                            } else {
                                0.0
                            }
                        };
                    }
                    0_usize
                };
            }
            0_usize
        };
        let _ = r4;
        let _ = {
            let total = rt.leaf_count();
            for i in 0..total {
                r16 = i;
                let _ = {
                    let total = rt.leaf_count();
                    for i in 0..total {
                        r17 = i;
                        let _ = {
                            let condition = {
                                let a0 = {
                                    let a0 = {
                                        let left = { let index = { let a0 = Some(r16); rt.matched_stroke_of(a0) }; match index { Some(index) => index as f64, None => -1.0 } };
                                        let right = 0.0_f64;
                                        left < right
                                    };
                                    !a0
                                };
                                let a1 = {
                                    let left = { let index = { let a0 = Some(r17); rt.matched_stroke_of(a0) }; match index { Some(index) => index as f64, None => -1.0 } };
                                    let right = 0.0_f64;
                                    left < right
                                };
                                a0 && a1
                            };
                            if condition {
                                {
                                {
                                    let a0 = {
                                        let left = {
                                            let left = {
                                                let left = {
                                                    let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                                    let right = {
                                                        let left = 0.5_f64;
                                                        let right = { let a0 = 0_usize; rt.total_leaf_cost(a0) };
                                                        left * right
                                                    };
                                                    left + right
                                                };
                                                let right = {
                                                    let left = 4.0_f64;
                                                    let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                                    left * right
                                                };
                                                left + right
                                            };
                                            let right = {
                                                let left = 4.0_f64;
                                                let right = { let a0 = 2_usize; rt.total_leaf_cost(a0) };
                                                left * right
                                            };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.push_state(a0);
                                }
                                r15 = { let a0 = Some(r16); rt.matched_stroke_of(a0) };
                                { let a0 = Some(r16); rt.uncommit(a0); }
                                { let a0 = Some(r17); rt.uncommit(a0); }
                                { let a0 = Some(r17); let a1 = r15; rt.commit_match(a0, a1); }
                                { let a0 = Some(r16); rt.commit_missing(a0); }
                                ({
                                    let a0 = {
                                        let left = {
                                            let left = {
                                                let left = {
                                                    let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                                    let right = {
                                                        let left = 0.5_f64;
                                                        let right = { let a0 = 0_usize; rt.total_leaf_cost(a0) };
                                                        left * right
                                                    };
                                                    left + right
                                                };
                                                let right = {
                                                    let left = 4.0_f64;
                                                    let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                                    left * right
                                                };
                                                left + right
                                            };
                                            let right = {
                                                let left = 4.0_f64;
                                                let right = { let a0 = 2_usize; rt.total_leaf_cost(a0) };
                                                left * right
                                            };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.keep_best_state(a0);
                                    0_usize
                                }) as f64
                            }
                            } else {
                                0.0
                            }
                        };
                    }
                    0_usize
                };
            }
            0_usize
        };
        let _ = r31;
        { let a0 = 2.0_f64; rt.push_state(a0); }
        let _ = -1.2964204826465524_f64;
        let _ = {
            let total = rt.leaf_count();
            for i in 0..total {
                r18 = i;
                let _ = {
                    let total = rt.leaf_count();
                    for i in 0..total {
                        r19 = i;
                        let _ = {
                            let condition = { let left = r18; let right = r19; left < right };
                            if condition {
                                {
                                {
                                    let a0 = {
                                        let left = {
                                            let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                            let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.push_state(a0);
                                }
                                { let a0 = Some(r18); let a1 = Some(r19); rt.swap_matches(a0, a1); }
                                ({
                                    let a0 = {
                                        let left = {
                                            let left = { let a0 = 1_usize; rt.total_leaf_cost(a0) };
                                            let right = { let a0 = 3_usize; rt.total_leaf_cost(a0) };
                                            left + right
                                        };
                                        let right = { let a0 = 1_usize; rt.total_pairwise_cost(a0) };
                                        left + right
                                    };
                                    rt.keep_best_state(a0);
                                    0_usize
                                }) as f64
                            }
                            } else {
                                0.0
                            }
                        };
                    }
                    0_usize
                };
            }
            0_usize
        };
        let _ = {
            let condition = ({
                let a0 = Some(6_usize);
                let a1 = Some(rt.merged_leaf_count());
                let before = { let a0 = 0_usize; rt.total_group_pairwise_cost(a0) };
                let trial = rt.begin_trial();
                if rt.try_swap_groups_repaired(a0, a1) { {
                        let after = { let a0 = 0_usize; rt.total_group_pairwise_cost(a0) };
                        if after >= before { rt.rollback(trial); }
                    } } else { drop(trial); }
                0_usize
            }) >= 1;
            if condition {
                {
                let total = rt.leaf_count();
                for i in 0..total {
                    r0 = i;
                    let _ = {
                        let total = rt.leaf_count();
                        for i in 0..total {
                            r31 = i as f64;
                            let _ = {
                                let a0 = {
                                    let a0 = { let a0 = Some(rt.open_stroke_count()); rt.nth_open_leaf(a0) };
                                    rt.matched_stroke_of(a0)
                                };
                                rt.nth_open_stroke(a0)
                            };
                        }
                        0_usize
                    };
                }
                0_usize
            }
            } else {
                {
                let a0 = 1_usize;
                let a1 = { let a0 = r1; rt.matched_stroke_of(a0) };
                let a2 = { let a0 = Some(r0); rt.nth_open_leaf(a0) };
                rt.swap_if_better(a0, a1, a2);
                0_usize
            }
            }
        };
        let _ = rt.open_stroke_count();
        { let a0 = rt.total_match_cost(); rt.keep_best_state(a0); }
        let _ = r18;
        let _ = 1.0_f64;
        let _ = 0.0_f64;
        let _ = r22;
        let _ = 6.0_f64;
        let _ = 0.0;
        r0 as f64
    };
}
