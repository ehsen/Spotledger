//! Currency DocType stub — Tier 3 financial core.

use spotledger_core::DocTypeMeta;
use spotledger_core::meta::{DocField, FieldType};
use spotledger_core::registry::MetaEntry;


pub fn item_meta() -> DocTypeMeta {
    DocTypeMeta {
            name:"Currency".into(),
            module:"Accounting".into(),

        is_child:false,
        is_single:false,
        is_submittable:false,
        is_tree:false,
        track_changes:true,

        fields:vec![
            DocField::new("curren_name","Currency Name",FieldType::Data)
                .required()
                .unique()
                .in_list()
                .in_standard_filter()


        ],

        permissions: vec![],
        title_field: None,
        search_fields: vec![],
        sort_field: None,
        sort_order: None,
        autoname: None,
        naming_series: None,
    }
}