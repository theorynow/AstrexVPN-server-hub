use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::features::nodes::domain::model::{HysteriaConfig, Node, XrayConfig};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct XrayConfigDto {
    pub port: u16,
    pub sni: String,
    pub public_key: String,
    pub short_id: String,
}

impl From<XrayConfig> for XrayConfigDto {
    fn from(c: XrayConfig) -> Self {
        Self {
            port: c.port,
            sni: c.sni,
            public_key: c.public_key,
            short_id: c.short_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HysteriaConfigDto {
    pub port: u16,
    pub sni: String,
}

impl From<HysteriaConfig> for HysteriaConfigDto {
    fn from(c: HysteriaConfig) -> Self {
        Self {
            port: c.port,
            sni: c.sni,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeDto {
    pub id: String,
    pub public_ip: String,
    pub name_en: String,
    pub name_ru: String,
    pub country_flag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xray: Option<XrayConfigDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hysteria: Option<HysteriaConfigDto>,
}

impl From<Node> for NodeDto {
    fn from(n: Node) -> Self {
        Self {
            id: n.id,
            public_ip: n.public_ip,
            name_en: n.name_en,
            name_ru: n.name_ru,
            country_flag: n.country_flag,
            xray: n.xray.map(Into::into),
            hysteria: n.hysteria.map(Into::into),
        }
    }
}
