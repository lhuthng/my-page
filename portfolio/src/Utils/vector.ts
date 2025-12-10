export default class Vect {
    x: number;
    y: number;
    constructor(x: number = 0, y: number = 0) {
        this.x = x;
        this.y = y;
    }
    clone() {
        return new Vect(this.x, this.y);
    }
    add(other: Vect) {
        this.x += other.x;
        this.y += other.y;
        return this;
    }
    sub(other: Vect) {
        this.x -= other.x;
        this.y -= other.y;
        return this;
    }
    neg() {
        this.x = -this.x;
        this.y = -this.y;
        return this;
    }
    mul(other: Vect) {
        this.x *= other.x;
        this.y *= other.y;
        return this;
    }
    mul2(value: number) {
        this.x *= value;
        this.y *= value;
        return this;
    }
    static mul(origin: Vect, other: Vect) {
        return origin.clone().mul(other);
    }
    static mul2(origin: Vect, value: number) {
        return origin.clone().mul2(value);
    }
}