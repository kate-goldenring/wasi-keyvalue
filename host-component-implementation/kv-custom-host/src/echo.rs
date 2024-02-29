use crate::table::Table;
use anyhow::Context;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{command, WasiCtx, WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!({
    path: "../wit",
    // interfaces: "../wit/deps/custom",
    world: "trigger",
    async: true
});

#[async_trait::async_trait]
impl wasi::keyvalue::types::HostStore for View {
    async fn get(
        &mut self,
        identifier: String,
    ) -> anyhow::Result<Result<wasmtime::component::Resource<wasi::keyvalue::types::Store>, Error>>
    {
        // TODO: Use ResourceTable
        match self.stores_id_to_handle.get(&identifier) {
            Some(s) => Ok(Ok(wasmtime::component::Resource::<
                wasi::keyvalue::types::Store,
            >::new_own(*s))),
            None => {
                let store = KVStore::new();
                let handle = self
                    .stores_handle_to_data
                    .push(store)
                    .expect("table is full");
                self.stores_id_to_handle.insert(identifier, handle);
                Ok(Ok(wasmtime::component::Resource::<
                    wasi::keyvalue::types::Store,
                >::new_own(handle)))
            }
        }
    }

    fn drop(
        &mut self,
        _rep: wasmtime::component::Resource<wasi::keyvalue::types::Store>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl wasi::keyvalue::types::Host for View {}

use wasi::keyvalue::types::Error;

#[async_trait::async_trait]
impl wasi::keyvalue::crud::Host for View {}

#[async_trait::async_trait]
impl wasi::keyvalue::crud::HostCrud for View {
    async fn open(
        &mut self,
        store: wasmtime::component::Resource<wasi::keyvalue::types::Store>,
    ) -> Result<wasmtime::component::Resource<wasi::keyvalue::crud::Crud>, anyhow::Error> {
        Ok(wasmtime::component::Resource::<wasi::keyvalue::crud::Crud>::new_own(store.rep()))
    }

    async fn get(
        &mut self,
        _self_: wasmtime::component::Resource<wasi::keyvalue::crud::Crud>,
        key: String,
    ) -> anyhow::Result<Result<Option<Vec<u8>>, Error>> {
        let store = self
            .stores_handle_to_data
            .get(_self_.rep())
            .ok_or(Error::NoSuchStore)?;
        let res = store
            .kv
            .read()
            .unwrap()
            .get(&key)
            .map(|k| k.as_bytes().to_owned());
        Ok(Ok(res))
    }

    async fn set(
        &mut self,
        _self_: wasmtime::component::Resource<wasi::keyvalue::crud::Crud>,
        key: String,
        val: Vec<u8>,
    ) -> anyhow::Result<Result<(), Error>> {
        let store = self
            .stores_handle_to_data
            .get(_self_.rep())
            .ok_or(Error::NoSuchStore)?;
        store
            .kv
            .write()
            .unwrap()
            .insert(key, String::from_utf8(val).unwrap());
        Ok(Ok(()))
    }

    async fn delete(
        &mut self,
        _self_: wasmtime::component::Resource<wasi::keyvalue::crud::Crud>,
        key: String,
    ) -> anyhow::Result<Result<(), Error>> {
        let store = self
            .stores_handle_to_data
            .get(_self_.rep())
            .ok_or(Error::NoSuchStore)?;
        store.kv.write().unwrap().remove(&key);
        Ok(Ok(()))
    }

    async fn exists(
        &mut self,
        _self_: wasmtime::component::Resource<wasi::keyvalue::crud::Crud>,
        key: String,
    ) -> anyhow::Result<Result<bool, Error>> {
        let store = self
            .stores_handle_to_data
            .get(_self_.rep())
            .ok_or(Error::NoSuchStore)?;
        let exists = store.kv.read().unwrap().contains_key(&key);
        Ok(Ok(exists))
    }

    async fn list_keys(
        &mut self,
        _self_: wasmtime::component::Resource<wasi::keyvalue::crud::Crud>,
    ) -> anyhow::Result<Result<Vec<String>, Error>> {
        let store = self
            .stores_handle_to_data
            .get(_self_.rep())
            .ok_or(Error::NoSuchStore)?;
        let keys = store.kv.read().unwrap().keys().cloned().collect();
        Ok(Ok(keys))
    }

    fn drop(
        &mut self,
        _rep: wasmtime::component::Resource<wasi::keyvalue::crud::Crud>,
    ) -> wasmtime::Result<()> {
        // call ResourceTable::delete
        Ok(())
    }
}

pub async fn echo(path: PathBuf, s: String) -> wasmtime::Result<String> {
    let mut config = Config::default();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    // Add the command world (aka WASI CLI) to the linker
    command::add_to_linker(&mut linker).context("Failed to link command world")?;
    wasi::keyvalue::types::add_to_linker(&mut linker, |x| x)
        .context("Failed to link keyvalue world")?;
    wasi::keyvalue::crud::add_to_linker(&mut linker, |x| x)
        .context("Failed to link keyvalue world")?;

    let view = View::new();

    let mut store = Store::new(&engine, view);

    let component = Component::from_file(&engine, path).context("Component file not found")?;

    let (instance, _) = Trigger::instantiate_async(&mut store, &component, &linker)
        .await
        .context("Failed to instantiate the world")?;
    instance
        .call_doing(&mut store, &s)
        .await
        .context("Failed to call doing function")
}
type KVMap = Arc<RwLock<HashMap<String, String>>>;
struct KVStore {
    kv: KVMap,
}

impl KVStore {
    fn new() -> Self {
        Self {
            kv: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

struct View {
    table: wasmtime::component::ResourceTable,
    ctx: WasiCtx,
    stores_id_to_handle: HashMap<String, u32>,
    stores_handle_to_data: Table<KVStore>,
    // store_handle_to_crud_handle: HashSet<u32>,
}

impl View {
    fn new() -> Self {
        let table = wasmtime::component::ResourceTable::default();
        let ctx = WasiCtxBuilder::new().inherit_stdout().build();

        Self {
            table,
            ctx,
            stores_id_to_handle: HashMap::new(),
            stores_handle_to_data: Table::new(1024),
        }
    }
}

impl WasiView for View {
    fn table(&self) -> &wasmtime::component::ResourceTable {
        &self.table
    }

    fn table_mut(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}
