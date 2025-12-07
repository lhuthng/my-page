import { clearLogin, saveLogin } from "$lib/client/user.js";

import { gsap } from "gsap";
import { Flip } from "gsap/Flip";

gsap.registerPlugin(Flip);

export function load({ data }) {
    const { user, accessToken } = data;
    if (user && accessToken) {
        const { username, displayName, role } = user;
        const { token, type } = accessToken;

        saveLogin({
            username,
            token,
            tokenType: type,
            displayName,
            role,
        });
    } else {
        clearLogin();
    }
}
