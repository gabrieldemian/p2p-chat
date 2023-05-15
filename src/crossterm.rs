// match &mut app.page {
//     Page::ChatRoom(page) => {
//         if let Ok(d) = rx.try_recv() {
//             match d {
//                 BkEvent::MessageReceived(msg) => {
//                     page.items.push(msg);
//                 }
//             }
//         }
//     }
// }
