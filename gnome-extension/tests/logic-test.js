// gjs -m gnome-extension/tests/logic-test.js
import System from 'system';

import {Throttle, UPDATE_INTERVAL_MS, stateForBrightness, stateForEffect, tileState} from '../logic.js';

let failures = 0;
function check(name, actual, expected) {
    const a = JSON.stringify(actual), e = JSON.stringify(expected);
    if (a === e) {
        print(`ok   ${name}`);
    } else {
        print(`FAIL ${name}: got ${a}, expected ${e}`);
        failures++;
    }
}

const ripple = ['Ripple', [0x3d, 0x38, 0x46], 52, 100];
const effects = [
    ['Off', false, false, 0, 0],
    ['Static', true, false, 0, 0],
    ['Wave', false, true, 10, 90],
    ['Ripple', true, true, 1, 100],
    ['Custom', false, false, 0, 0],
];
const connected = {deviceState: 'connected', lightsOn: true, gameActive: false, state: ripple};

check('lit shows the effect', tileState(connected), {checked: true, sensitive: true, subtitle: 'Ripple'});
check('game shows Gaming Layout', tileState({...connected, gameActive: true}),
    {checked: true, sensitive: true, subtitle: 'Gaming Layout'});
check('off has no subtitle', tileState({...connected, lightsOn: false}),
    {checked: false, sensitive: true, subtitle: null});
check('off during a game has no subtitle', tileState({...connected, lightsOn: false, gameActive: true}),
    {checked: false, sensitive: true, subtitle: null});
check('Custom shows Custom', tileState({...connected, state: ['Custom', [0, 0, 0], 50, 80]}).subtitle, 'Custom');
check('disconnected', tileState({...connected, deviceState: 'disconnected'}),
    {checked: false, sensitive: false, subtitle: 'Disconnected'});
check('no permission', tileState({...connected, deviceState: 'permission-denied'}).subtitle, 'No Permission');
check('unsupported', tileState({...connected, deviceState: 'unsupported'}).subtitle, 'Unsupported Keyboard');
check('unknown device state reads as disconnected', tileState({...connected, deviceState: 'odd'}).subtitle,
    'Disconnected');

const red30 = ['Static', [255, 0, 0], 30, 70];
check('effect keeps colour and brightness', stateForEffect('Ripple', effects, red30), ['Ripple', [255, 0, 0], 30, 70]);
check('speed clamped up', stateForEffect('Wave', effects, ['Static', [255, 0, 0], 5, 70]),
    ['Wave', [255, 0, 0], 10, 70]);
check('speed clamped down', stateForEffect('Wave', effects, ['Static', [255, 0, 0], 95, 70]),
    ['Wave', [255, 0, 0], 90, 70]);
check('no speed: saved speed kept', stateForEffect('Static', effects, ripple), ['Static', [0x3d, 0x38, 0x46], 52, 100]);
check('unknown effect passes through', stateForEffect('Nope', effects, ripple)[0], 'Nope');

check('brightness replaced', stateForBrightness(ripple, 20), ['Ripple', [0x3d, 0x38, 0x46], 52, 20]);
check('brightness rounded and clamped', [stateForBrightness(ripple, 49.6)[3], stateForBrightness(ripple, 140)[3],
    stateForBrightness(ripple, -3)[3]], [50, 100, 0]);

// The slider dragged from 100 to 20 over one second, an update every 16 ms,
// with the timer flushing at each interval boundary after a send.
function simulate(updates) {
    const t = new Throttle(UPDATE_INTERVAL_MS);
    const sent = [];
    let flushAt = null;
    const runFlushes = until => {
        while (flushAt !== null && flushAt <= until) {
            const v = t.flush(flushAt);
            if (v !== undefined) {
                sent.push([flushAt, v]);
                flushAt += UPDATE_INTERVAL_MS;
            } else {
                flushAt = null;
            }
        }
    };
    for (const [at, value] of updates) {
        runFlushes(at);
        const v = t.push(at, value);
        if (v !== undefined)
            sent.push([at, v]);
        flushAt ??= at + UPDATE_INTERVAL_MS;
    }
    runFlushes(Infinity);
    return sent;
}
const drag = [];
for (let i = 0; i <= 62; i++)
    drag.push([i * 16, Math.round(100 - 80 * i / 62)]);
const sent = simulate(drag);
const gaps = sent.slice(1).map(([at], i) => at - sent[i][0]);
check('drag: at most one send per interval', gaps.every(g => g >= UPDATE_INTERVAL_MS), true);
check('drag: about ten sends a second', sent.length <= 11 && sent.length >= 9, true);
check('drag: the final value is sent', sent.at(-1)[1], 20);
check('single change sent at once', simulate([[0, 40]]), [[0, 40]]);

const t = new Throttle(UPDATE_INTERVAL_MS);
t.push(0, 1);
check('busy within the interval', t.busy(50), true);
check('idle after it', t.busy(150), false);
t.push(150, 2);
t.push(160, 3);
check('busy while holding', t.busy(400), true);
check('flush gives the newest held value', t.flush(250), 3);
check('nothing left to flush', t.flush(350), undefined);

if (failures) {
    print(`${failures} failed`);
    System.exit(1);
}
print('all passed');
