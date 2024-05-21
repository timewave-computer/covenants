use localic_std::transactions::ChainRequestBuilder;

use crate::base::TestContext;

impl TestContext {
    pub fn get_transfer_channels(&self) -> TestContextQuery {
        TestContextQuery::new(self, QueryType::TransferChannel)
    }

    pub fn get_connections(&self) -> TestContextQuery {
        TestContextQuery::new(self, QueryType::Connection)
    }

    pub fn get_ccv_channels(&self) -> TestContextQuery {
        TestContextQuery::new(self, QueryType::CCVChannel)
    }

    pub fn get_ibc_denoms(&self) -> TestContextQuery {
        TestContextQuery::new(self, QueryType::IBCDenom)
    }

    pub fn get_admin_addr(&self) -> TestContextQuery {
        TestContextQuery::new(self, QueryType::AdminAddr)
    }

    pub fn get_native_denom(&self) -> TestContextQuery {
        TestContextQuery::new(self, QueryType::NativeDenom)
    }

    pub fn get_request_builder(&self) -> TestContextQuery {
        TestContextQuery::new(self, QueryType::RequestBuilder)
    }
}

pub enum QueryType {
    TransferChannel,
    Connection,
    CCVChannel,
    IBCDenom,
    AdminAddr,
    NativeDenom,
    RequestBuilder,
}

pub struct TestContextQuery<'a> {
    context: &'a TestContext,
    query_type: QueryType,
    src_chain: Option<String>,
    dest_chain: Option<String>,
    contract_name: Option<String>,
}

impl<'a> TestContextQuery<'a> {
    pub fn new(context: &'a TestContext, query_type: QueryType) -> Self {
        Self {
            context,
            query_type,
            src_chain: None,
            dest_chain: None,
            contract_name: None,
        }
    }

    pub fn src(mut self, src_chain: &str) -> Self {
        self.src_chain = Some(src_chain.to_string());
        self
    }

    pub fn dest(mut self, dest_chain: &str) -> Self {
        self.dest_chain = Some(dest_chain.to_string());
        self
    }

    pub fn contract(mut self, contract_name: &str) -> Self {
        self.contract_name = Some(contract_name.to_string());
        self
    }

    pub fn get(self) -> String {
        let query_response = match self.query_type {
            QueryType::TransferChannel => self.get_transfer_channel(),
            QueryType::Connection => self.get_connection_id(),
            QueryType::CCVChannel => self.get_ccv_channel(),
            QueryType::IBCDenom => self.get_ibc_denom(),
            QueryType::AdminAddr => self.get_admin_addr(),
            QueryType::NativeDenom => self.get_native_denom(),
            _ => None,
        };
        query_response.unwrap()
    }

    pub fn get_all(self) -> Vec<String> {
        match self.query_type {
            QueryType::TransferChannel => self.get_all_transfer_channels(),
            QueryType::Connection => self.get_all_connections(),
            _ => vec![],
        }
    }

    pub fn get_request_builder(mut self, chain: &str) -> &'a ChainRequestBuilder {
        self.src_chain = Some(chain.to_string());
        let rb = match self.query_type {
            QueryType::RequestBuilder => self.get_rb(),
            _ => None,
        };
        rb.unwrap()
    }

    fn get_transfer_channel(self) -> Option<String> {
        if let (Some(ref src), Some(ref dest)) = (self.src_chain, self.dest_chain) {
            self.context.transfer_channel_ids.get(&(src.clone(), dest.clone())).cloned()
        } else {
            None
        }
    }

    fn get_all_transfer_channels(self) -> Vec<String> {
        if let Some(ref src) = self.src_chain {
            self.context
                .transfer_channel_ids
                .iter()
                .filter(|((s, _), _)| s == src)
                .map(|(_, v)| v.clone())
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    }

    fn get_connection_id(self) -> Option<String> {
        if let (Some(ref src), Some(ref dest)) = (self.src_chain, self.dest_chain) {
            self.context.connection_ids.get(&(src.clone(), dest.clone())).cloned()
        } else {
            None
        }
    }

    fn get_all_connections(self) -> Vec<String> {
        if let Some(ref src) = self.src_chain {
            self.context
                .connection_ids
                .iter()
                .filter(|((s, _), _)| s == src)
                .map(|(_, v)| v.clone())
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    }

    fn get_ccv_channel(self) -> Option<String> {
        if let (Some(ref src), Some(ref dest)) = (self.src_chain, self.dest_chain) {
            self.context.ccv_channel_ids.get(&(src.clone(), dest.clone())).cloned()
        } else {
            None
        }
    }

    fn get_ibc_denom(self) -> Option<String> {
        if let (Some(ref src), Some(ref dest)) = (self.src_chain, self.dest_chain) {
            self.context.ibc_denoms.get(&(src.clone(), dest.clone())).cloned()
        } else {
            None
        }
    }

    fn get_admin_addr(self) -> Option<String> {
        if let Some(ref src) = self.src_chain {
            self.context.chains.get(src).map(|chain| chain.admin_addr.clone())
        } else {
            None
        }
    }

    fn get_native_denom(self) -> Option<String> {
        if let Some(ref src) = self.src_chain {
            self.context.chains.get(src).map(|chain| chain.native_denom.clone())
        } else {
            None
        }
    }

    fn get_rb(self) -> Option<&'a ChainRequestBuilder> {
        if let Some(ref src) = self.src_chain {
            self.context.chains.get(src).map(|chain| &chain.rb)
        } else {
            None
        }
    }
}
