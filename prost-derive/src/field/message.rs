use failure::Error;
use syn::Meta;
use quote::Tokens;

use field::{
    word_attr,
    tag_attr,
    set_option,
    set_bool,
    Label,
};

#[derive(Clone)]
pub struct Field {
    pub label: Label,
    pub tag: u32,
}

impl Field {
    pub fn new(attrs: &[Meta], default_tag: Option<u32>) -> Result<Option<Field>, Error> {
        let mut message = false;
        let mut label = None;
        let mut tag = None;
        let mut boxed = false;

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("message", attr) {
                set_bool(&mut message, "duplicate message attribute")?;
            } else if word_attr("boxed", attr) {
                set_bool(&mut boxed, "duplicate boxed attribute")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !message {
            return Ok(None);
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute for message field: {:?}", unknown_attrs[0]),
            _ => bail!("unknown attributes for message field: {:?}", unknown_attrs),
        }

        if tag.is_none() {
            tag = default_tag;
        }
        let tag = match tag {
            Some(tag) => tag,
            None => bail!("message field is missing a tag attribute"),
        };

        Ok(Some(Field {
            label: label.unwrap_or(Label::Optional),
            tag: tag,
        }))
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(attrs, None)? {
            if let Some(attr) = attrs.iter().find(|attr| Label::from_attr(attr).is_some()) {
                bail!("invalid atribute for oneof field: {}", attr.name());
            }
            field.label = Label::Required;
            Ok(Some(field))
        } else {
            Ok(None)
        }
    }

    pub fn encode(&self, ident: Tokens) -> Tokens {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                if let Some(ref msg) = #ident {
                    _prost::encoding::message::encode(#tag, msg, buf);
                }
            },
            Label::Required => quote! {
                _prost::encoding::message::encode(#tag, &#ident, buf);
            },
            Label::Repeated => quote! {
                for msg in &#ident {
                    _prost::encoding::message::encode(#tag, msg, buf);
                }
            },
        }
    }

    pub fn merge(&self, ident: Tokens) -> Tokens {
        match self.label {
            Label::Optional => quote! {
                _prost::encoding::message::merge(wire_type,
                                                 #ident.get_or_insert_with(Default::default),
                                                 buf)
            },
            Label::Required => quote! {
                _prost::encoding::message::merge(wire_type, &mut #ident, buf)
            },
            Label::Repeated => quote! {
                _prost::encoding::message::merge_repeated(wire_type, &mut #ident, buf)
            },
        }
    }

    pub fn encoded_len(&self, ident: Tokens) -> Tokens {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                #ident.as_ref().map_or(0, |msg| _prost::encoding::message::encoded_len(#tag, msg))
            },
            Label::Required => quote! {
                _prost::encoding::message::encoded_len(#tag, &#ident)
            },
            Label::Repeated => quote! {
                _prost::encoding::message::encoded_len_repeated(#tag, &#ident)
            },
        }
    }

    pub fn clear(&self, ident: Tokens) -> Tokens {
        match self.label {
            Label::Optional => quote!(#ident = ::std::option::Option::None),
            Label::Required => quote!(#ident.clear()),
            Label::Repeated => quote!(#ident.clear()),
        }
    }
}
