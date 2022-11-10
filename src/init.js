(() => {
    // Prevent user to reload the page
    document.addEventListener("keydown", (e) => {
        if (e.key === "F5" ||
            (e.ctrlKey && e.key === "r") ||
            (e.ctrlKey && e.key === "R")) {
            e.preventDefault();
        }
    });

    // Disable WebView default context menu
    document.addEventListener("contextmenu", (e) => {
        // 关闭右键contextmenu
        // e.preventDefault();
    });

    // Window events
    const MAIN_BUTTON = 1;
    const DOUBLE_CLICK = 2;
    window.addEventListener("load", () => {
        // Title bar events
        const drag_zone = document.getElementById("drag_zone");
        drag_zone.addEventListener('mousedown', (e) => {
            if (e.buttons === MAIN_BUTTON) {
                if (e.detail === DOUBLE_CLICK) {
                    window.rpc.notify('toggle_maximize');
                } else {
                    window.rpc.notify('drag_window');
                }
            }
        })

        const minimize = document.getElementById("minimize");
        minimize.addEventListener("click", () => {
            window.rpc.notify("minimize");
        });

        const maximize = document.getElementById("maximize");
        maximize.addEventListener("click", () => {
            window.rpc.notify("toggle_maximize");
        });

        document.addEventListener("tse_maximized_state_changed", (e) => {
            if (e.detail.is_maximized === true) {
                maximize.classList.add("maximized");
            } else {
                maximize.classList.remove("maximized");
            }
        });

        document.addEventListener("on_message", e => {
            console.log('onmessage', e);
        });

        const close = document.getElementById("close");
        close.addEventListener("click", () => {
            window.rpc.notify("close");
        });

        // Show the window when initialized
        window.rpc.notify("init");
    });
})();

if (typeof(window) == "undefined") {
    window = {}
}

// (function (window) {

//     const RPCState = {
//         NOT_CONNECTED : 0,
//         CONNECTING : 1,
//         CONNECTED : 2
//     }

//     class RPC {

//         constructor() {
//             this._callbacks = {};
//             this._callbackId = 0;
//             this.proxy = undefined;
//             this._state = RPCState.NOT_CONNECTED;
//         }

//         connect() {
//             this._state = RPCState.CONNECTING;
//             return new Promise((resolve, reject) => {
//                 new QWebChannel(qt.webChannelTransport, (channel) => {
//                     //console.log("on qwebchancel connected", qt.webChannelTransport, channel);
//                     this.proxy = channel.objects.proxy;

//                     this.proxy.on_message.connect(this._onMessage.bind(this));
//                     resolve(this.proxy);
//                     this._state = RPCState.CONNECTED;
//                 });
//             });
//         }

//         invokeSync(method, params, callback) {
//             //console.log("RPC", "invoke", "method", method, "params", params);
//             //return new Promise((resolve, reject) => {
//                 if (typeof(this.proxy) == "undefined") {
//                     this.connect().then((_) => {
//                         this.proxy.invoke(method, JSON.stringify(params), callback);
//                     });
//                 } else {
//                     this.proxy.invoke(method, JSON.stringify(params), callback);
//                 }
//             //});
//         }

//         invoke(method, params, callback) {
//             return new Promise((resolve, reject) => {
//                 this.invokeCallback(method, params, (result) => {
//                     resolve(result);
//                 });
//             });
//         }

//         invokeCallback(method, params, callback) {
//             const callbackId = ++this._callbackId;
//             this._callbacks[callbackId] = callback;
//             //console.log("RPC", "invoke", "method=", method, ",params=", params, ",callbackId=", callbackId);

//             if (this._state == RPCState.NOT_CONNECTED) {
//                 this.connect().then((_) => {
//                     this.proxy.post_message(callbackId, method, JSON.stringify(params));
//                 });
//             } else if (this._state == RPCState.CONNECTING) {
//                 setTimeout(() => {
//                     if (this.proxy) {
//                         this.proxy.post_message(callbackId, method, JSON.stringify(params));
//                     }
//                 }, 300); // TODO: wait connecting
//             } else if (this._state == RPCState.CONNECTED) {
//                 this.proxy.post_message(callbackId, method, JSON.stringify(params));
//             } else {
//                 console.error("invalid state", this._state);
//             }
//         }

//         _onMessage(message) {
//             message = message && JSON.parse(message);
//             //console.log("RPC", "onMessage", message);
//             if (!message) {
//                 console.error("rpc response message is null");
//             }
//             if (typeof(this._callbacks[[message.callback_id]]) != "undefined") { // response from rpc call
//                 const callback = this._callbacks[[message.callback_id]];
//                 callback(message.result);
//                 delete this._callbacks[[message.callback_id]];
//             } else { // broadcast
//                 var event = document.createEvent('Event');
//                 event.initEvent('message', false, true);
//                 event.data = message.result;
//                 window.dispatchEvent(event); // window.addEventListener('message', event => { });
//             }
//         }
//     }

//     window.RPC = new RPC();
//     //required for use with nodejs
//     if (typeof module === 'object') {
//         module.exports.rpc = window.rpc;
//     }

// })(window);