// Decisions for the tile that need no Shell: what it shows, what a menu
// choice sends, and when the slider may send. Tested with gjs alone
// (tests/logic-test.js).

/** At most one brightness update per this many milliseconds, like the app. */
export const UPDATE_INTERVAL_MS = 100;

const DEVICE_SUBTITLES = {
    'disconnected': 'Disconnected',
    'permission-denied': 'No Permission',
    'unsupported': 'Unsupported Keyboard',
};

/**
 * What the toggle shows.
 *
 * @param {object} props the service's values: deviceState, lightsOn,
 *   gameActive and state ([effect, [r, g, b], speed, brightness])
 * @returns {{checked: boolean, sensitive: boolean, subtitle: ?string}}
 */
export function tileState(props) {
    if (props.deviceState !== 'connected') {
        const subtitle = DEVICE_SUBTITLES[props.deviceState] ?? DEVICE_SUBTITLES['disconnected'];
        return {checked: false, sensitive: false, subtitle};
    }
    if (!props.lightsOn)
        return {checked: false, sensitive: true, subtitle: null};
    const subtitle = props.gameActive ? 'Gaming Layout' : props.state?.[0] ?? null;
    return {checked: true, sensitive: true, subtitle};
}

/**
 * The `SetState` arguments for choosing `name` in the menu: the saved
 * colour and brightness, and the saved speed clamped to the effect's range.
 *
 * @param {string} name the chosen effect
 * @param {Array} effects the service's [name, acceptsColor, acceptsSpeed, min, max] list
 * @param {Array} state the saved [effect, [r, g, b], speed, brightness]
 * @returns {Array} [effect, [r, g, b], speed, brightness]
 */
export function stateForEffect(name, effects, state) {
    const [, color, speed, brightness] = state;
    const info = effects.find(e => e[0] === name);
    if (!info || !info[2])
        return [name, color, speed, brightness];
    const [, , , min, max] = info;
    return [name, color, Math.min(Math.max(speed, min), max), brightness];
}

/**
 * The `SetState` arguments for a new brightness, everything else saved.
 *
 * @param {Array} state the saved [effect, [r, g, b], speed, brightness]
 * @param {number} brightness 0 to 100
 * @returns {Array} [effect, [r, g, b], speed, brightness]
 */
export function stateForBrightness(state, brightness) {
    const [effect, color, speed] = state;
    return [effect, color, speed, Math.min(Math.max(Math.round(brightness), 0), 100)];
}

/**
 * Sends at most one value per interval; the newest value in between is
 * held until `flush`. A port of the app's Rust `Throttle`.
 */
export class Throttle {
    constructor(intervalMs) {
        this._interval = intervalMs;
        this._lastSent = null;
        this._held = undefined;
    }

    /**
     * A new value. Returns it if it should be sent now; otherwise it is
     * held (replacing any older held value) and `undefined` is returned.
     */
    push(now, value) {
        if (this._lastSent !== null && now - this._lastSent < this._interval) {
            this._held = value;
            return undefined;
        }
        this._lastSent = now;
        this._held = undefined;
        return value;
    }

    /** Called once the interval has passed: the held value, if any. */
    flush(now) {
        const value = this._held;
        if (value === undefined)
            return undefined;
        this._held = undefined;
        this._lastSent = now;
        return value;
    }

    /** Whether the user is mid-change: something held, or sent within the interval. */
    busy(now) {
        return this._held !== undefined ||
            (this._lastSent !== null && now - this._lastSent < this._interval);
    }
}
