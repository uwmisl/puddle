import * as CodeMirror from "codemirror";
import "codemirror/lib/codemirror.css";
import "codemirror/mode/javascript/javascript.js";

import m from "mithril";
import Board from "./visualizer";

let rustModule = import("../pkg/index.js");

const INITIAL_CODE = `// feel free to edit this javascript

let d1 = sys.create({
    location: {y: 1, x: 1},
    vol: 1.0,
    dim: {y: 1, x: 1},
});
let d2 = sys.create({
    vol: 1.0,
    dim: {y: 1, x: 1},
});
let d = sys.mix(d1, d2);
let [a, b] = sys.split(d);
console.log("flush", sys.flush());
`;

// the mithril component
let Editor = (storeEditorFunction) => ({
    oncreate: function(vnode) {
        let editor = CodeMirror(vnode.dom, {
            value: INITIAL_CODE,
            mode: "javascript",
            lineNumbers: true,
        });
        storeEditorFunction(editor);
        m.redraw();
    },
    view: () => m("div"),
});

function App() {

    // the actual codemirror object
    let editor;

    // this compenent will save the editor in the editor variable when created
    let MyEditor = Editor((ed) => {editor = ed;});

    let data;

    function runCode() {
        console.log("clicked");
        console.log(editor.getValue());
        rustModule.then(module => {
            let sys = module.System.new();
            try {
                let f = new Function('sys', editor.getValue());
                f(sys)
                sys.flush();
                data = sys.getLogs();
            }
            catch (e) {
                console.warn(e);
                let match = e.stack.match(/(Function|<anonymous>):(\d+)/);
                let line = match[2];
                alert(`${e.toString()} at ${line}`);
            }
            console.log("Generated data", data);
        })
    };

    return {
        view: function() {
            return m('div', [
                m(MyEditor),
                m('button', {onclick: runCode}, 'Run'),
                m(Board, { data } ),
            ])
        }
    };
}

m.mount(document.body, App);
