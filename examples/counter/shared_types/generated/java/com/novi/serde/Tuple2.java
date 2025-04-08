// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde;

import java.util.Objects;

public final class Tuple2<T0, T1> {
    public final T0 field0;
    public final T1 field1;

    public Tuple2(T0 f0, T1 f1) {
        Objects.requireNonNull(f0, "f0 must not be null");
        Objects.requireNonNull(f1, "f1 must not be null");
        this.field0 = f0;
        this.field1 = f1;
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        Tuple2<?,?> other = (Tuple2) obj;
        if (!Objects.equals(this.field0, other.field0)) {
            return false;
        }
        if (!Objects.equals(this.field1, other.field1)) {
            return false;
        }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.field0 != null ? this.field0.hashCode() : 0);
        value = 31 * value + (this.field1 != null ? this.field1.hashCode() : 0);
        return value;
    }

}
