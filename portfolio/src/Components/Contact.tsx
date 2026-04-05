import Me from "@/Components/SVGR/Me";
import gsap from "gsap";
import { useEffect, useReducer, useRef, useState } from "react";
import { useBoundedEase } from "@/Utils/Hooks/ease";
import Vect from "@/Utils/vector";
import "@/Styles/Contact.css";

interface Transform {
  position: Vect;
  size: Vect;
}

interface MeState {
  lastOrigin: Transform;
  getOrigin: () => Transform;
  children: Record<string, Transform>;
}

type Action =
  | { type: "set-get-origin"; payload: { getDomRect: () => DOMRect } }
  | { type: "add"; payload: { key: string; domRect: DOMRect } };

const initialState: MeState = {
  lastOrigin: { position: new Vect(), size: new Vect() },
  getOrigin: () => ({ position: new Vect(), size: new Vect() }),
  children: {},
};

function reducer(state: MeState, { type, payload }: Action) {
  switch (type) {
    case "set-get-origin": {
      state.getOrigin = () => {
        const domRect = payload.getDomRect();
        return {
          position: new Vect(domRect.left, domRect.top),
          size: new Vect(domRect.width, domRect.height),
        };
      };
      state.lastOrigin = state.getOrigin();
      break;
    }
    case "add": {
      state.children[payload.key] = {
        position: new Vect(payload.domRect.left, payload.domRect.top).sub(
          state.lastOrigin.position,
        ),
        size: new Vect(payload.domRect.width, payload.domRect.height),
      };
      break;
    }
  }

  return state;
}

function findLocal(
  state: MeState,
  position: Vect,
  child?: string,
  offset?: {
    byValue?: { x: number; y: number };
    byPercent?: { x: number; y: number };
  },
): Vect {
  const result = state.getOrigin().position.sub(position);
  if (!child || !state.children[child]) {
    return result;
  }
  result.add(state.children[child].position);

  if (offset?.byValue) {
    result.add(new Vect(offset.byValue.x, offset.byValue.y));
  }
  if (offset?.byPercent) {
    result.add(
      Vect.mul(
        state.children[child].size,
        new Vect(offset.byPercent.x, offset.byPercent.y),
      ),
    );
  }
  return result;
}

interface MovementProps {
  key: string;
  influence: number;
}

const movements: MovementProps[] = [
  { key: "left-eye", influence: 0.7 },
  { key: "right-eye", influence: 0.7 },
  { key: "glasses", influence: 0.5 },
  { key: "eyebrows", influence: 0.4 },
  { key: "smile", influence: 0.4 },
  { key: "nose", influence: 0.4 },
  { key: "blushs", influence: 0.4 },
  { key: "face-base", influence: 0.3 },
  { key: "ears", influence: 0.2 },
  { key: "forehair", influence: 0.25 },
  { key: "hair", influence: 0.2 },
];

export default function Contact() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const lookAtMouse = useRef<(event: MouseEvent) => void>(null);
  const stopLookingAtMouse = useRef<() => void>(null);
  const emailRef = useRef<HTMLInputElement>(null);
  const messageRef = useRef<HTMLTextAreaElement>(null);
  const [getStatus, setStatus] = useState<String | null>(null);
  const [getOk, setOk] = useState<boolean>(true);

  useEffect(() => {
    const melement = document.querySelector("#me-svg");
    if (!melement) return;

    dispatch({
      type: "set-get-origin",
      payload: { getDomRect: () => melement.getBoundingClientRect() },
    });

    const groups = movements
      .map(({ key, ...others }) => {
        const element = melement.querySelector(`#${key}`);
        return element && { key, element, ...others };
      })
      .filter((group) => group !== null);

    groups.forEach(({ key, element }) => {
      dispatch({
        type: "add",
        payload: { key, domRect: element.getBoundingClientRect() },
      });
    });

    const boundedEase = useBoundedEase(4, 100, 2);

    lookAtMouse.current = (event: MouseEvent) => {
      const delta = findLocal(
        state,
        new Vect(event.clientX, event.clientY),
        "eyes",
        { byPercent: { x: 0.5, y: 0.5 } },
      ).neg();
      const dist = Math.sqrt(delta.x * delta.x + delta.y * delta.y);
      const easedDist = boundedEase(dist);
      delta.mul2(easedDist / dist);

      groups.forEach(({ element, influence }) => {
        const _delta = Vect.mul2(delta, influence);
        gsap.to(element, {
          translateX: _delta.x,
          translateY: _delta.y,
          duration: 0.5,
        });
      });
    };

    stopLookingAtMouse.current = () => {
      groups.forEach(({ element }) => {
        gsap.to(element, {
          translateX: 0,
          translateY: 0,
          duration: 0.5,
        });
      });
    };

    const tl = gsap
      .timeline({
        repeat: -1,
        repeatDelay: 3,
      })
      .to("#eyes", {
        scaleY: 0.2,
        transformOrigin: "center bottom",
        duration: 0.1,
      })
      .to("#eyes", {
        scaleY: 1,
        transformOrigin: "center bottom",
        duration: 0.1,
      });

    return () => {
      tl.kill();
      if (lookAtMouse.current)
        window.removeEventListener("mousemove", lookAtMouse.current);
    };
  }, []);

  const assignMouseMove = () => {
    if (!lookAtMouse.current) return;
    window.addEventListener("mousemove", lookAtMouse.current);
  };

  const assignMouseLeave = () => {
    if (lookAtMouse.current) {
      window.removeEventListener("mousemove", lookAtMouse.current);
    }

    stopLookingAtMouse.current?.();
  };

  return (
    <div
      id="contact-form"
      className="w-full bg-orange-chalk"
      onMouseEnter={assignMouseMove}
      onMouseLeave={assignMouseLeave}
    >
      <div className="py-20 w-[min(75rem,100%-2rem)] mx-auto">
        <div className="flex flex-col md:flex-row gap-10 md:gap 0 relative items-center bg-white-chalk w-full px-6 pt-20 md:pt-0 md:px-20 h-auto md:h-100 rounded-2xl shadow-2xl shadow-white-chalk-dark">
          <div className="absolute text-4xl sm:text-6xl text-nowrap text-blackboard hover:text-darkboard bg-white-chalk hover:bg-yellow-chalk outline-white-chalk hover:outline-yellow-chalk outline-2 outline-offset-2 outline-double -top-8 drop-shadow-2xl px-2 transition-all duration-100">
            Hi there!
          </div>
          <div className="w-full md:w-[60%] text-blackboard text-lg space-y-2">
            <p>
              - Got a project in mind, a question about my work, or just want to
              connect? Drop me a message — I read every one and I'll get back to
              you.
            </p>
            <form
              className="bg-white-chalk-dark/35 py-3 px-4 rounded-lg"
              onSubmit={async (e) => {
                e.preventDefault();

                if (!emailRef.current || !messageRef.current) {
                  console.log("Nah");
                  return;
                }

                const email = emailRef.current.value,
                  message = messageRef.current.value;

                const res = await fetch(
                  "https://blog.huuthangle.site/api/mail/contact-form",
                  {
                    method: "POST",
                    headers: {
                      "Content-Type": "application/json",
                    },
                    body: JSON.stringify({
                      name: "portfolio",
                      email,
                      content: message,
                    }),
                  },
                );

                setOk(res.ok);
                setStatus(await res.text());
              }}
            >
              <div className="flex gap-2">
                <label htmlFor="email">Email: </label>
                <input
                  ref={emailRef}
                  className="px-2 flex-1 border-b-2 border-white-chalk-dark/35 focus:outline-hidden focus:border-orange-chalk focus:bg-white-chalk-dark/30 transition-all duration-100"
                  type="email"
                  id="email"
                  placeholder="Your contact point"
                  autoComplete="off"
                  required
                />
              </div>
              <div>
                <label htmlFor="message">Message:</label>
                <br />
                <textarea
                  ref={messageRef}
                  className="scrollbar-custom scroll-thumb-orange px-2 w-full h-22 lg:h-15 border-l-2 border-white-chalk-dark/35 focus:outline-hidden focus:border-orange-chalk focus:bg-white-chalk-dark/30 transition-all duration-100 resize-none"
                  id="message"
                  rows={2}
                  minLength={5}
                  maxLength={512}
                  placeholder="Your words, my inbox, instant magic."
                  spellCheck="false"
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  required
                ></textarea>
                <br />
              </div>
              <div className="flex flex-col items-end">
                <button
                  className="relative submit-button border-2 px-2 py-1 border-white-chalk-dark hover:border-orange-chalk hover:font-bold hover:-translate-x-1 hover:-translate-y-1 active:border-orange-chalk active:font-bold active:translate-0 transition-all duration-100 cursor-pointer whitespace-nowrap"
                  type="submit"
                >
                  Send Message
                </button>
                {getStatus !== null && (
                  <span
                    className={`text-right ${getOk ? "text-green-600" : "text-red-600"} `}
                  >
                    {getStatus}
                  </span>
                )}
              </div>
            </form>
          </div>
          <div className="flex justify-center w-full md:w-[40%] drop-shadow-lg">
            <Me id="me-svg" className="w-40 h-auto overflow-visible" />
          </div>
        </div>
      </div>
    </div>
  );
}
