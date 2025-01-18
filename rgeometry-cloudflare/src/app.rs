use std::sync::{Arc, Mutex};

use crate::wasm::Wasm;
use leptos::{html::Img, prelude::*};
use leptos_meta::*;
use rfd::*;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="utf-8"/>
            <meta name="viewport" content="width=device-width, initial-scale=1"/>

            <AutoReload options=options.clone() />
            <HydrationScripts options/>
            <MetaTags/>
        </head>
        <body class="bg-sky-100">
            <App/>
        </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let img_ref = NodeRef::new();

    let wasm = Arc::new(Mutex::new(None));

    if !cfg!(feature = "ssr") {
        let wasm = wasm.clone();
        leptos::task::spawn_local(async move {
            log::info!("Opening file dialog...");
            let Some(file) = AsyncFileDialog::new().pick_file().await else {
                // return Err("No file selected".to_string());
                return;
            };
            let bytes = file.read().await;

            // Create Wasm instance from the loaded bytes and wrap in mutex
            let mut wasm = wasm.lock().unwrap();
            *wasm = Some(Wasm::new(&bytes).unwrap());
        });
    }

    // // Create a resource that fetches data and creates a Wasm instance
    // let wasm_resource = LocalResource::new(|| async move {
    //     log::info!("Opening file dialog...");
    //     let Some(file) = AsyncFileDialog::new().pick_file().await else {
    //         return Err("No file selected".to_string());
    //     };
    //     let bytes = file.read().await;

    //     // Create Wasm instance from the loaded bytes and wrap in mutex
    //     let wasm = std::sync::Mutex::new(
    //         Wasm::new(&bytes).map_err(|e| format!("Failed to create Wasm instance: {}", e))?,
    //     );

    //     Ok(wasm)
    // });

    // let img = RwSignal::new(String::new());

    fn animate(node: NodeRef<Img>, wasm: Arc<Mutex<Option<Wasm>>>) {
        {
            let mut wasm = wasm.lock().unwrap();
            if let Some(wasm) = wasm.as_mut() {
                if let Some(img) = node.get_untracked() {
                    img.set_src(&format!("data:image/svg+xml,{}", wasm.render()));
                }
            }
        }
        request_animation_frame(move || animate(node, wasm));
    }
    if !cfg!(feature = "ssr") {
        request_animation_frame(move || animate(img_ref, wasm));
    }

    view! {
        <div class="p-4">
            <h1 class="text-2xl font-bold mb-4">"RGeometry WASM Viewer"</h1>
            <img node_ref=img_ref/>
            // <Suspense
            //     fallback=move || view! { <p>"Loading WASM file..."</p> }
            // >
            //     {move || Suspend::new(async move {
            //         match img.get() {
            //             img => view! {
            //                 <div class="space-y-4">
            //                     // <div class="bg-white p-4 rounded shadow">
            //                     //     <h2 class="font-bold">"Schema:"</h2>
            //                     //     <pre class="bg-gray-100 p-2 rounded">{schema}</pre>
            //                     // </div>

            //                     <div class="bg-white p-4 rounded shadow">
            //                         <h2 class="font-bold">"Output:"</h2>
            //                         <pre class="bg-gray-100 p-2 rounded">
            //                             <img src={format!("data:image/svg+xml,{}", img)}/>
            //                         </pre>
            //                     </div>
            //                 </div>
            //             }.into_any(),
            //             // Err(err) => view! {
            //             //     <p class="text-red-500">"Error: " {err}</p>
            //             // }.into_any(),
            //         }
            //     })}
            // </Suspense>
        </div>
    }
}
