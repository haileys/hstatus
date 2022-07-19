use std::path::Path;

use futures::Stream;
use tokio::sync::watch;
use tokio::io::unix::AsyncFd;

use crate::util::wpactrl;

pub fn ssid(control_path: &Path) -> impl Stream<Item = Option<String>> {
    let (tx, rx) = watch::channel(None);

    let attach = wpactrl::Client::builder()
        .ctrl_path(control_path)
        .open()
        .expect("open error")
        .attach()
        .expect("attach error");

    let attach = AsyncFd::new(attach).expect("asyncfd error");

    tokio::spawn(async move {
        if let Err(e) = run_client(attach, tx).await {
            eprintln!("wifi: client terminated with error: {:?}", e);
        }
    });

    tokio_stream::wrappers::WatchStream::new(rx)
}

async fn run_client(
    mut client: AsyncFd<wpactrl::ClientAttached>,
    tx: watch::Sender<Option<String>>,
) -> Result<(), wpactrl::Error> {
    use tokio::task::block_in_place;

    block_in_place(|| client.get_mut().send_request("STATUS").unwrap());

    loop {
        let mut readable = client.readable_mut().await?;

        let mut received_unsolicited = false;

        while let Some(msg) = readable.get_inner_mut().recv()? {
            if is_message_unsolicited(&msg) {
                received_unsolicited = true;
            } else {
                // we only ever send status command, so all replies must be
                // statuses. parse for SSID:

                let ssid = msg.lines()
                    .find(|line| line.starts_with("ssid="))
                    .map(|line| line[5..].trim());

                tx.send(ssid.map(str::to_owned));
            }
        }

        readable.clear_ready();

        if received_unsolicited {
            block_in_place(|| client.get_mut().send_request("STATUS").unwrap());
        }
    }
}

fn is_message_unsolicited(msg: &str) -> bool {
    // logic taken from hostapd/src/command/wpa_ctrl.c
    // wpa_ctrl_request function

    msg.starts_with("<") || msg.starts_with("IFNAME=")
}
