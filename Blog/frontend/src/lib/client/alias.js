import { writable } from "svelte/store";

export class Alias {
    constructor(text, origin, action) {
        this.text = text;
        this.origin = origin;
        this.action = action;
    }

    static create(newAlias) {
        return new Alias(newAlias, undefined, "created");
    }

    modify(newAlias) {
        if (this.action !== "created") {
            this.action = "modified";
        }
        this.text = newAlias;
    }

    willBeDeleted() {
        return this.origin !== undefined;
    }
}
