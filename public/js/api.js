/*
 * feed-me admin — API client
 *
 * All calls hit the JSON API under /api and rely on a session cookie that the
 * backend sets on POST /api/login (HttpOnly, scoped to /api). We never see or
 * store a token; the browser sends the cookie automatically.
 *
 * When the backend answers 401 (cookie missing/expired) we emit a global
 * `unauthorized` event; app.js listens for it and drops the user on the login
 * screen.
 *
 * Endpoints marked (WIP) are not implemented server-side yet — the UI is wired
 * to the intended shape and will light up as the backend catches up.
 */
(function (global) {
    "use strict";

    class ApiError extends Error {
        constructor(status, body) {
            super(typeof body === "string" && body ? body : `request failed (${status})`);
            this.name = "ApiError";
            this.status = status;
            this.body = body;
        }
    }

    async function request(method, path, body) {
        const opts = {
            method,
            credentials: "same-origin",
            headers: { Accept: "application/json" },
        };
        if (body !== undefined) {
            opts.headers["Content-Type"] = "application/json";
            opts.body = JSON.stringify(body);
        }

        let res;
        try {
            res = await fetch("/api" + path, opts);
        } catch (e) {
            throw new ApiError(0, "network error — is the server running?");
        }

        if (res.status === 401) {
            global.dispatchEvent(new CustomEvent("unauthorized"));
            throw new ApiError(401, "session expired");
        }

        const text = await res.text();
        let data = null;
        if (text) {
            try { data = JSON.parse(text); } catch { data = text; }
        }

        if (!res.ok) {
            throw new ApiError(res.status, data);
        }
        return data;
    }

    function qs(params) {
        if (!params) return "";
        const p = new URLSearchParams();
        Object.entries(params).forEach(([k, v]) => {
            if (v !== undefined && v !== null && v !== "") p.append(k, v);
        });
        const s = p.toString();
        return s ? "?" + s : "";
    }

    global.api = {
        ApiError,

        // auth
        login: (username, password) => request("POST", "/login", { username, password }),
        logout: () => request("POST", "/logout"),

        // feeds
        feeds: {
            list: () => request("GET", "/feed/"),
            get: (id) => request("GET", `/feed/${id}`),
            create: (data) => request("POST", "/feed/", data),
            update: (id, data) => request("PUT", `/feed/${id}`, data),          // (WIP)
            remove: (id) => request("DELETE", `/feed/${id}`),                   // (WIP)
        },

        // feed entries (WIP) — nested under a feed
        entries: {
            list: (feedId, params) => request("GET", `/feed/${feedId}/entries${qs(params)}`),
            create: (feedId, data) => request("POST", `/feed/${feedId}/entries`, data),
            update: (feedId, id, data) => request("PUT", `/feed/${feedId}/entries/${id}`, data),
            remove: (feedId, id) => request("DELETE", `/feed/${feedId}/entries/${id}`),
        },

        // users (WIP)
        users: {
            list: () => request("GET", "/users/"),
            create: (data) => request("POST", "/users/", data),
            update: (id, data) => request("PUT", `/users/${id}`, data),
            remove: (id) => request("DELETE", `/users/${id}`),
        },
    };
})(window);
