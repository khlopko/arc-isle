use yaml_rust::{Yaml};
use std::fmt::{Debug, Display, Formatter};
use crate::parser::utils::{as_str_or, YamlHash};
use crate::schema::{Host, Hosts};

pub struct HostsParser<'a> {
    pub main: &'a Yaml
}

impl<'a> HostsParser<'a> {
    pub fn parse(&self) -> Result<Hosts, HostsError> {
        let raw_hosts: Option<&YamlHash> = self.main["hosts"].as_hash();
        let raw_hosts: &YamlHash = raw_hosts.ok_or(HostsError::NotFound)?;
        let mut hosts = Hosts::new();
        for pair in raw_hosts {
            let (key, value): (&Yaml, &Yaml) = pair;
            let host = self.host(key, value)?;
            hosts.push(host);
        }
        Ok(hosts)
    }

    fn host(&self, key: &Yaml, value: &Yaml) -> Result<Host, HostsError> {
        let env = as_str_or(key, HostsError::MissingEnv)?;
        let address = as_str_or(
            value,
            HostsError::MissingAddress(String::from(&env))
        )?;
        Ok(Host { env, address })
    }
}

pub enum HostsError {
    NotFound,
    MissingEnv,
    MissingAddress(String)
}

impl HostsError {
    fn default_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HostsError::NotFound =>
                write!(f, "No hosts were specified."),
            HostsError::MissingEnv =>
                write!(f, "Missing environment key for host."),
            HostsError::MissingAddress(env) =>
                write!(f, "Missing address value for environment: {}", env),
        }
    }
}

impl std::error::Error for HostsError {
}

impl Display for HostsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}

impl Debug for HostsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}
