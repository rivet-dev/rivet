var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSaxophoneFire.js
var require_faSaxophoneFire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSaxophoneFire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "saxophone-fire";
    var width = 640;
    var height = 512;
    var aliases = ["sax-hot"];
    var unicode = "f8db";
    var svgPathData = "M256 153.6c0-25.5-12.4-67.2-59.5-113.9c-9.4-9.3-24.5-8.6-33.8 .8l-9.1 9.1L116.8 12.7C112 7.9 105.6 5.3 99.2 5.4c-4.7 0-9.4 1.6-13.4 4.7C50 37.9 0 92.3 0 153.6c0 7.6 .7 15.1 2 22.4l77.6 0c7.7-23.3 29.5-44.6 41-54.6c4.3-3.7 10.5-3.7 14.8 0c11.6 10 33.4 31.3 41 54.6l77.6 0c1.3-7.3 2-14.8 2-22.4zM180.1 329.8L229.3 256l2.7 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L24 208c-13.3 0-24 10.7-24 24c0 9.5 5.5 17.7 13.5 21.6L2.7 313C.9 323 0 333.1 0 343.2l0 7C0 439.6 72.4 512 161.8 512c49.4 0 96.1-22.6 126.8-61.4L534.4 140.1c6.1-7.7 15.3-12.1 25.1-12.1l64.2 0c9 0 16.3-7.3 16.3-16.3c0-4.9-2.2-9.5-5.9-12.6L609.8 78.8c-11.5-9.6-26-14.8-41-14.8l-30.3 0c-17 0-33.3 6.7-45.3 18.7L449 127l-8-8c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l8 8L401 175l-8-8c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l8 8L353 223l-8-8c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l8 8L214.6 361.4c-4.2 4.2-10 6.6-16 6.6c-12.5 0-22.6-10.1-22.6-22.6l0-1.9c0-4.8 1.4-9.6 4.1-13.6zM144 280a24 24 0 1 1 -48 0 24 24 0 1 1 48 0zM88 344a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faSaxophoneFire = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faSaxophoneFire.js
var import_faSaxophoneFire = __toESM(require_faSaxophoneFire());
var export_faSaxophoneFire = import_faSaxophoneFire.definition;
export {
  export_faSaxophoneFire as faSaxophoneFire
};
