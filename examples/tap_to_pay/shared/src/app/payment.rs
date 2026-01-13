use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Default, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Payment {
    pub amount: u32, // pence,
    pub status: PaymentStatus,
}

impl Payment {
    #[must_use]
    pub fn new(amount: u32) -> Self {
        Payment {
            amount,
            status: PaymentStatus::New,
        }
    }

    pub fn start(&mut self) {
        if self.status == PaymentStatus::New && self.amount > 0 {
            self.status = PaymentStatus::PendingTap;
        }
    }

    pub fn send(&mut self) {
        if self.status == PaymentStatus::PendingTap {
            self.status = PaymentStatus::Sent;
        }
    }

    pub fn confirm_send(&mut self) {
        if self.status == PaymentStatus::Sent {
            self.status = PaymentStatus::Completed(Receipt::default());
        }
    }

    pub fn receipt(&mut self) -> Option<&mut Receipt> {
        if let PaymentStatus::Completed(receipt) = &mut self.status {
            Some(receipt)
        } else {
            None
        }
    }
}

#[derive(Facet, Default, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[repr(C)]
pub enum PaymentStatus {
    #[default]
    New,
    PendingTap,
    Sent,
    Completed(Receipt),
    Failed(String),
}

#[derive(Facet, Default, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Receipt {
    pub email: String,
    pub status: ReceiptStatus,
}

impl Receipt {
    pub fn send(&mut self) {
        if self.status == ReceiptStatus::New {
            self.status = ReceiptStatus::Pending;
        }
    }

    pub fn confirm_send(&mut self) {
        if self.status == ReceiptStatus::Pending {
            self.status = ReceiptStatus::Sent;
        }
    }
}

#[derive(Facet, Default, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[repr(C)]
pub enum ReceiptStatus {
    #[default]
    New,
    Pending,
    Sent,
}
