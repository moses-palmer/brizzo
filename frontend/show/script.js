const CANVAS_ID = 'canvas';
const BASE_URL = (window.location + "")
    .replace(/[^/]*$/, "") + "../api/" + window.location.hash.substring(1);
const MARGIN = 1.0;

const UNKNOWN = true;


// The identifiers from the starting room up until the current one
const trail = [];

// Our cached rooms
const rooms = {};


const main = async () => {
    let current = await begin();
    while (current) {
        const xid = current.see.find(xid => rooms[xid] === UNKNOWN);
        if (xid !== undefined) {
            trail.push(current);
            current = await move(xid);
        } else if (trail.length > 0) {
            while (trail.length > 0) {
                const back = trail.pop();
                if (current.see.indexOf(back.xid) >= 0) {
                    current = await move(back.xid);
                    break;
                }
            }
        } else {
            break;
        }
    }
};


/**
 * Starts over.
 */
const begin = () => req("GET", undefined, true).then(cache);


/**
 * Moves to a neighbouring room.
 */
const move = (xid) => req("PUT", {xid: xid}).then(cache).then(paint);


/**
 * Adds a room to the cache and updates the view.
 */
const cache = (room) => {
    if (room && rooms[room.xid] === undefined
            || rooms[room.xid] === UNKNOWN) {
        trail.push(room.xid)
        rooms[room.xid] = room;
        room.see.forEach(remember);
    }
    return room;
};


/**
 * Marks a room as known.
 */
const remember = (xid) => {
    if (rooms[xid] === undefined) {
        rooms[xid] = UNKNOWN;
    }
};


/**
 * Performs a request.
 */
const req = (method, data, reset) => fetch(BASE_URL, init(method, data, reset))
    .then(r => {
        if (r.ok) {
            return r.text();
        } else {
            throw r;
        }
    }).catch(e => {
        switch (e.status) {
        case 404:
            alert("Not found!");
                break;
        default:
            alert(method + " " + BASE_URL
                + (data !== undefined
                    ? " <" + JSON.stringify(data) + ">"
                    : "")
                + ": "
                + e.status);
            break;
        }
    }).then(JSON.parse);


/**
 * Generates a fetch configuration.
 */
const init = (method, data, reset) => {
    const result = {
        credentials: reset === true
            ? "omit"
            : "include",
        method: method}
    if (data !== undefined) {
        result.body = JSON.stringify(data);
        result.headers = {
            "Content-Type": "application/json"};
    }
    return result;
};


/**
 * Paints a room.
 */
const paint = (room) => {
    const canvas = document.getElementById(CANVAS_ID);

    // Create a polygon from the corner points
    const el = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "polygon");
    el.setAttribute("points", room.pos.map(p => `${p.x},${p.y}`).join(" "));
    el.style.fill = room.col;

    // Update the viewbox
    let x = canvas.viewBox.baseVal.x;
    let y = canvas.viewBox.baseVal.y;
    let width = canvas.viewBox.baseVal.width;
    let height = canvas.viewBox.baseVal.height;
    room.pos.forEach(pos => {
        x = Math.min(
            x,
            pos.x - MARGIN);
        y = Math.min(
            y,
            pos.y - MARGIN);
        width = Math.max(
            width,
            pos.x - x + MARGIN);
        height = Math.max(
            height,
            pos.y - y + MARGIN);
    });
    canvas.setAttribute(
        "viewBox",
        x + " " + y + " " + width + " " + height);

    canvas.appendChild(el);

    return room;
};
