use futures::future;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use zbus::dbus_proxy;
use zbus::zvariant::OwnedObjectPath;

use crate::util;

#[dbus_proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager",
)]
trait NetworkManager {
    #[dbus_proxy(property)]
    fn primary_connection(&self) -> zbus::Result<OwnedObjectPath>;
}

#[dbus_proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager",
)]
trait ActiveConnection {
    #[dbus_proxy(property)]
    fn id(&self) -> zbus::Result<String>;
}

pub fn network() -> impl Stream<Item = Option<String>> {
    let primary_conn_stream = util::stream::from_future(async move {
        let dbus = zbus::Connection::system().await.unwrap();

        let nm = NetworkManagerProxy::builder(&dbus)
            .build()
            .await
            .unwrap();

        primary_connection(nm).await
    });

    let primary_name_stream = primary_conn_stream
        .and_then(|conn| async move { Ok(connection_name(conn).await) })
        .map(util::stream::flatten_result_stream);

    util::stream::follow_latest(primary_name_stream)
        .map(|result| result
            .map_err(|e| eprintln!("source::wifi::networkmanager: {:?}", e))
            .ok())
}

async fn primary_connection(proxy: NetworkManagerProxy<'_>) -> impl Stream<Item = zbus::Result<ActiveConnectionProxy<'_>>> + '_ {
    let stream = proxy.receive_primary_connection_changed().await
        .then(|change| async move { change.get().await });

    let dbus = proxy.connection().clone();
    let conn = proxy.primary_connection().await;

    stream::once(future::ready(conn))
        .chain(stream)
        .and_then(move |path| {
            let dbus = dbus.clone();
            async move {
                Ok(ActiveConnectionProxy::builder(&dbus)
                    .path(path)?
                    .build()
                    .await?)
            }
        })
}

async fn connection_name(proxy: ActiveConnectionProxy<'_>) -> impl Stream<Item = zbus::Result<String>> + '_ {
    let stream = proxy.receive_id_changed().await
        .then(|change| async move { change.get().await });

    let name = proxy.id().await;

    stream::once(future::ready(name))
        .chain(stream)
}
