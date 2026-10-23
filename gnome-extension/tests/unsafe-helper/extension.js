// Only for the nested test Shell started by tests/nested.sh, which has its
// own session bus and in-memory settings. Unsafe mode lets the test call
// org.gnome.Shell.Eval and org.gnome.Shell.Screenshot.
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class UnsafeHelper extends Extension {
    enable() {
        global.context.unsafe_mode = true;
    }

    disable() {
        global.context.unsafe_mode = false;
    }
}
