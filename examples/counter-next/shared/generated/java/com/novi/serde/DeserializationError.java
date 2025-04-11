// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde;

@SuppressWarnings("serial")
public final class DeserializationError extends Exception {
    public DeserializationError(String s) {
        super(s);
    }
}
