//! One capability that is `Tasks` and `FormSource`: the bootstrap of a connection answers both on the one socket.
//!
//! Pattern: Decorator — the same shape as `Controlled` of fs-rpc (`Control` next to the program's capability): the calls of
//! `FormSource` go to its capability, every other call goes, unchanged, to `Tasks`.

use std::rc::Rc;

use capnp::any_pointer;
use capnp::capability::{Client, DispatchCallResult, FromServer, Params, Promise, Results, Server};
use capnp::traits::HasTypeId;
use fs_interfaces::formsource_capnp::form_source;

struct Both {
    tasks: Client,
    form: Client,
}

#[derive(Clone)]
struct Dispatch(Rc<Both>);

impl FromServer<Both> for Client {
    type Dispatch = Dispatch;
    fn from_server(s: Rc<Both>) -> Dispatch {
        Dispatch(s)
    }
}

impl std::ops::Deref for Dispatch {
    type Target = Both;
    fn deref(&self) -> &Both {
        &self.0
    }
}

impl Server for Dispatch {
    fn dispatch_call(
        self,
        interface_id: u64,
        method_id: u16,
        params: Params<any_pointer::Owned>,
        mut results: Results<any_pointer::Owned>,
    ) -> DispatchCallResult {
        let to = if interface_id == <form_source::Client as HasTypeId>::TYPE_ID {
            &self.0.form
        } else {
            &self.0.tasks
        };
        let mut request =
            to.new_call::<any_pointer::Owned, any_pointer::Owned>(interface_id, method_id, None);
        DispatchCallResult::new(
            Promise::from_future(async move {
                request.get().set_as(params.get()?)?;
                let response = request.send().promise.await?;
                results.get().set_as(response.get()?)?;
                Ok(())
            }),
            false,
        )
    }

    fn as_ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
}

/// One capability that is `Tasks` and `FormSource`.
pub(crate) fn both(tasks: Client, form: Client) -> Client {
    capnp_rpc::new_client::<Client, _>(Both { tasks, form })
}
