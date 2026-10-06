mod gw {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub struct Identity(pub spacetimedb::Identity);

    impl spacetimedb::SpacetimeType for Identity {
        fn make_type<S: spacetimedb::spacetimedb_lib::sats::typespace::TypespaceBuilder>(
            typespace: &mut S,
        ) -> spacetimedb::spacetimedb_lib::AlgebraicType {
            <spacetimedb::Identity as spacetimedb::SpacetimeType>::make_type(typespace)
        }
    }

    impl spacetimedb::Serialize for Identity {
        fn serialize<S: spacetimedb::spacetimedb_lib::ser::Serializer>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            <spacetimedb::Identity as spacetimedb::Serialize>::serialize(&self.0, serializer)
        }
    }

    impl<'de> spacetimedb::Deserialize<'de> for Identity {
        fn deserialize<D: spacetimedb::spacetimedb_lib::de::Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Self, D::Error> {
            <spacetimedb::Identity as spacetimedb::Deserialize>::deserialize(deserializer).map(Self)
        }
    }

    #[allow(non_camel_case_types)]
    pub struct receive_ping;

    impl spacetimedb::rt::ExplicitNames for receive_ping {}

    impl spacetimedb::rt::FnInfo for receive_ping {
        type Invoke = spacetimedb::rt::ReducerFn;
        type FnKind = spacetimedb::rt::FnKindReducer;
        const NAME: &'static str = "receive_ping";
        const ARG_NAMES: &'static [Option<&'static str>] = &[Some("target"), Some("body")];
        const INVOKE: Self::Invoke = |_, _| unreachable!("metadata only");
    }
}

#[spacetimedb::table(accessor = outbound_ping, outbox(gw::receive_ping))]
struct OutboundPing {
    #[primary_key]
    #[auto_inc]
    id: u64,
    target: gw::Identity,
    body: String,
}

fn main() {}
