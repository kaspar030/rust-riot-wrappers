/// This module provides a wrapper around a coap_handler::Handler that can be registered at a RIOT
/// GcoapHandler.
use core::convert::TryInto;

use coap_message::{MutableWritableMessage, ReadableMessage};

use crate::coap_message::ResponseMessage;
use crate::gcoap::PacketBuffer;

/// Adapter to get a [crate::gcoap::Handler] from a more generic [coap_handler::Handler], typically
/// to register it through a [crate::gcoap::SingleHandlerListener].
pub struct GcoapHandler<H>(pub H)
where
    H: coap_handler::Handler;

impl<H> crate::gcoap::Handler for GcoapHandler<H>
where
    H: coap_handler::Handler,
{
    fn handle(&mut self, pkt: &mut PacketBuffer) -> isize {
        let request_data = self.0.extract_request_data(pkt);
        let mut lengthwrapped = ResponseMessage::new(pkt);
        self.0.build_response(&mut lengthwrapped, request_data);
        lengthwrapped.finish()
    }
}

impl<H> crate::gcoap::WithLinkEncoder for GcoapHandler<H>
where
    H: coap_handler::Handler + coap_handler::Reporting,
{
    fn encode(&self, writer: &mut crate::gcoap::LinkEncoder) {
        use coap_handler::Record;
        for record in self.0.report() {
            writer.write_comma_maybe();
            writer.write(b"<");
            for pathelement in record.path() {
                writer.write(b"/");
                writer.write(pathelement.as_ref().as_bytes());
            }
            writer.write(b">");
            if let Some(rel) = record.rel() {
                // Not trying to be smart about whether or not we need the quotes
                writer.write(b";rel=\"");
                writer.write(rel.as_bytes());
                writer.write(b"\"");
            }
            for attr in record.attributes() {
                use coap_handler::Attribute::*;
                match attr {
                    Observable => writer.write(b";obs"),
                    Interface(i) => {
                        writer.write(b";if=\"");
                        writer.write(i.as_bytes());
                        writer.write(b"\"");
                    }
                    ResourceType(r) => {
                        writer.write(b";rt=\"");
                        writer.write(r.as_bytes());
                        writer.write(b"\"");
                    }
                    // FIXME: deduplicate with what's somewhere in coap-handler-implementations;
                    // implement remaining items
                    _ => (),
                }
            }
        }
    }
}

/// Blanket implementation for mutex wrapped resources
///
/// This is useful in combination with the defauilt implementation for Option as well.
impl<'b, H> coap_handler::Handler for &'b crate::mutex::Mutex<H>
where
    H: coap_handler::Handler,
{
    type RequestData = Option<H::RequestData>;

    fn extract_request_data<'a>(&mut self, request: &'a impl ReadableMessage) -> Self::RequestData {
        self.try_lock().map(|mut h| h.extract_request_data(request))
    }

    fn estimate_length(&mut self, request: &Self::RequestData) -> usize {
        if let Some(r) = request {
            if let Some(mut s) = self.try_lock() {
                return s.estimate_length(r);
            }
        }

        1
    }

    fn build_response(
        &mut self,
        response: &mut impl MutableWritableMessage,
        request: Self::RequestData,
    ) {
        if let Some(r) = request {
            if let Some(mut s) = self.try_lock() {
                return s.build_response(response, r);
            }
        }

        response.set_code(
            coap_numbers::code::SERVICE_UNAVAILABLE
                .try_into()
                .map_err(|_| "Message type can't even exprss Service Unavailable")
                .unwrap(),
        );
        response.set_payload(b"");
    }
}
