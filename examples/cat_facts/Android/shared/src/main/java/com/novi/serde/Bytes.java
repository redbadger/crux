// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde;

import java.util.Arrays;
import java.util.Objects;

/**
 * Immutable wrapper class around byte[].
 *
 * Enforces value-semantice for `equals` and `hashCode`.
 */
public final class Bytes {
    private final byte[] content;

    private static final Bytes EMPTY = new Bytes(new byte[0]);

    /// Low-level constructor (use with care).
    public Bytes(byte[] content) {
        Objects.requireNonNull(content, "content must not be null");
        this.content = content;
    }

    public static Bytes empty() {
        return EMPTY;
    }

    public static Bytes valueOf(byte[] content) {
        Objects.requireNonNull(content, "content must not be null");
        if (content.length == 0) {
            return EMPTY;
        } else {
            return new Bytes(content.clone());
        }
    }

    public byte[] content() {
        return this.content.clone();
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        Bytes other = (Bytes) obj;
        return Arrays.equals(this.content, other.content);
    }

    public int hashCode() {
        return Arrays.hashCode(content);
    }

}
