pub const ZSA_VENDOR_ID: u16 = 0x3297;
pub const VOYAGER_PRODUCT_ID: u16 = 0x1977;
pub const RAW_HID_USAGE_PAGE: u16 = 0xFF60;
pub const RAW_HID_USAGE: u16 = 0x61;

/// A supported keyboard model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    Voyager,
}

impl Model {
    pub fn from_ids(vendor_id: u16, product_id: u16) -> Option<Model> {
        match (vendor_id, product_id) {
            (ZSA_VENDOR_ID, VOYAGER_PRODUCT_ID) => Some(Model::Voyager),
            _ => None,
        }
    }

    /// The model, if this HID interface is a supported keyboard's raw-HID interface.
    pub fn from_interface(
        vendor_id: u16,
        product_id: u16,
        usage_page: u16,
        usage: u16,
    ) -> Option<Model> {
        if usage_page == RAW_HID_USAGE_PAGE && usage == RAW_HID_USAGE {
            Model::from_ids(vendor_id, product_id)
        } else {
            None
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Model::Voyager => "Voyager",
        }
    }

    /// The `geometry` argument Oryx's GraphQL API expects.
    pub fn oryx_geometry(self) -> &'static str {
        match self {
            Model::Voyager => "voyager",
        }
    }
}
