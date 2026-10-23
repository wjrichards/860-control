// Quick Settings tile for the g512d lighting service. Everything goes
// through the service's D-Bus interface; the extension keeps no settings
// and never starts the service.

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {QuickMenuToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Slider} from 'resource:///org/gnome/shell/ui/slider.js';

import {Throttle, UPDATE_INTERVAL_MS, stateForBrightness, stateForEffect, tileState} from './logic.js';

const BUS_NAME = 'io.github.g512control.Lighting1';
const OBJECT_PATH = '/io/github/g512control/Lighting1';
const APP_DESKTOP_ID = 'io.github.g512control.Control.desktop';
const ICON = 'input-keyboard-symbolic';

// The members the tile uses; the service has more.
const LightingProxy = Gio.DBusProxy.makeProxyWrapper(`
<node>
  <interface name="io.github.g512control.Lighting1">
    <property name="DeviceState" type="s" access="read"/>
    <property name="DeviceName" type="s" access="read"/>
    <property name="Effects" type="a(sbbyy)" access="read"/>
    <property name="State" type="(s(yyy)yy)" access="read"/>
    <property name="GameSwitch" type="b" access="read"/>
    <property name="GameActive" type="b" access="read"/>
    <property name="LightsOn" type="b" access="read"/>
    <method name="SetState">
      <arg name="effect" type="s" direction="in"/>
      <arg name="color" type="(yyy)" direction="in"/>
      <arg name="speed" type="y" direction="in"/>
      <arg name="brightness" type="y" direction="in"/>
      <arg name="pending" type="b" direction="out"/>
    </method>
    <method name="SetGameSwitch">
      <arg name="on" type="b" direction="in"/>
    </method>
    <method name="SetLightsOn">
      <arg name="on" type="b" direction="in"/>
    </method>
  </interface>
</node>`);

const BrightnessItem = GObject.registerClass(
class BrightnessItem extends PopupMenu.PopupBaseMenuItem {
    constructor() {
        super({activate: false});
        this.add_child(new St.Icon({icon_name: 'display-brightness-symbolic', style_class: 'popup-menu-icon'}));
        this.slider = new Slider(0);
        this.slider.accessible_name = 'Keyboard Brightness';
        this.add_child(this.slider);
    }
});

const LightingToggle = GObject.registerClass(
class LightingToggle extends QuickMenuToggle {
    constructor() {
        super({
            title: 'Keyboard',
            iconName: ICON,
            menuButtonAccessibleName: 'Open keyboard lighting menu',
            visible: false,
        });

        this._throttle = new Throttle(UPDATE_INTERVAL_MS);
        this._flushId = 0;
        this._effectNames = null;
        this._effectItems = [];
        this._syncing = false;

        this.menu.setHeader(ICON, 'Keyboard Lighting', '');

        this._effectsSection = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._effectsSection);
        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        this._brightness = new BrightnessItem();
        this.menu.addMenuItem(this._brightness);
        this._brightness.slider.connect('notify::value', () => this._brightnessChanged());

        this._gameItem = new PopupMenu.PopupSwitchMenuItem('Gaming Layout During Games', false);
        this._gameItem.connect('toggled', (item, on) => {
            if (!this._syncing)
                this._call('SetGameSwitch', on);
        });
        this.menu.addMenuItem(this._gameItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this.menu.addSettingsAction('Lighting Settings', APP_DESKTOP_ID);

        this.connect('clicked', () => this._call('SetLightsOn', !this.checked));

        this._proxy = new LightingProxy(Gio.DBus.session, BUS_NAME, OBJECT_PATH, (proxy, error) => {
            if (!this._proxy)
                return; // disabled before the proxy was ready
            if (error) {
                console.warn(`G512 tile: ${error.message}`);
                return;
            }
            this._sync();
        }, null, Gio.DBusProxyFlags.DO_NOT_AUTO_START);
        this._propsId = this._proxy.connect('g-properties-changed', () => this._sync());
        this._ownerId = this._proxy.connect('notify::g-name-owner', () => this._sync());
    }

    _props() {
        const p = this._proxy;
        return {
            deviceState: p.DeviceState ?? 'disconnected',
            deviceName: p.DeviceName ?? '',
            effects: p.Effects ?? [],
            state: p.State,
            gameSwitch: p.GameSwitch ?? false,
            gameActive: p.GameActive ?? false,
            lightsOn: p.LightsOn ?? true,
        };
    }

    _sync() {
        if (!this._proxy)
            return;
        // Hidden while the service is not on the bus, or too old to know
        // about the lights setting.
        this.visible = this._proxy.g_name_owner !== null && this._proxy.LightsOn !== null;
        if (!this.visible) {
            this.menu.close();
            return;
        }
        const props = this._props();
        const tile = tileState(props);
        this.set({checked: tile.checked, reactive: tile.sensitive, subtitle: tile.subtitle});
        this.menu.setHeader(ICON, 'Keyboard Lighting', props.deviceName);

        this._syncing = true;
        this._syncEffects(props);
        // Mid-drag the slider leads; the flush timer re-syncs once it stops.
        if (props.state && !this._throttle.busy(GLib.get_monotonic_time() / 1000))
            this._brightness.slider.value = props.state[3] / 100;
        this._brightness.visible = props.state !== null && props.state !== undefined;
        this._gameItem.setToggleState(props.gameSwitch);
        this._syncing = false;
    }

    _syncEffects(props) {
        const names = props.effects.map(e => e[0]);
        if (JSON.stringify(names) !== JSON.stringify(this._effectNames)) {
            this._effectNames = names;
            this._effectsSection.removeAll();
            this._effectItems = names.map(name => {
                const item = new PopupMenu.PopupMenuItem(name);
                item.connect('activate', () => this._chooseEffect(name));
                this._effectsSection.addMenuItem(item);
                return item;
            });
        }
        const current = props.lightsOn ? props.state?.[0] : null;
        this._effectItems.forEach((item, i) => {
            item.setOrnament(names[i] === current ? PopupMenu.Ornament.CHECK : PopupMenu.Ornament.NONE);
        });
    }

    _chooseEffect(name) {
        const props = this._props();
        if (props.state)
            this._call('SetState', ...stateForEffect(name, props.effects, props.state));
    }

    _brightnessChanged() {
        if (this._syncing)
            return;
        const value = Math.round(this._brightness.slider.value * 100);
        const now = GLib.get_monotonic_time() / 1000;
        const ready = this._throttle.push(now, value);
        if (ready !== undefined)
            this._sendBrightness(ready);
        this._scheduleFlush();
    }

    _scheduleFlush() {
        if (this._flushId)
            return;
        this._flushId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, UPDATE_INTERVAL_MS, () => {
            this._flushId = 0;
            const held = this._throttle.flush(GLib.get_monotonic_time() / 1000);
            if (held !== undefined) {
                this._sendBrightness(held);
                this._scheduleFlush();
            } else {
                // The user has stopped: show what the service has.
                this._sync();
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    _sendBrightness(value) {
        const state = this._props().state;
        if (state)
            this._call('SetState', ...stateForBrightness(state, value));
    }

    _call(method, ...args) {
        if (!this._proxy)
            return;
        this._proxy[`${method}Async`](...args).catch(e => {
            console.warn(`G512 tile: ${method} failed: ${e.message}`);
            this._sync();
        });
    }

    destroy() {
        if (this._flushId) {
            GLib.source_remove(this._flushId);
            this._flushId = 0;
        }
        this._proxy.disconnect(this._propsId);
        this._proxy.disconnect(this._ownerId);
        this._proxy = null;
        super.destroy();
    }
});

const LightingIndicator = GObject.registerClass(
class LightingIndicator extends SystemIndicator {
    constructor() {
        super();
        this.quickSettingsItems.push(new LightingToggle());
    }

    destroy() {
        this.quickSettingsItems.forEach(item => item.destroy());
        this.quickSettingsItems = [];
        super.destroy();
    }
});

export default class G512LightingExtension extends Extension {
    enable() {
        this._indicator = new LightingIndicator();
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
    }
}
