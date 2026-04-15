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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceExhaling.js
var require_faFaceExhaling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceExhaling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-exhaling";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e480";
    var svgPathData = "M256 512c55.8 0 107.4-17.9 149.5-48.2c-10.5-9.3-17.9-22.1-20.5-36.5l-42.2-12.7C329.3 410.6 320 398.1 320 384s9.3-26.6 22.8-30.6L385 340.7c5.3-30 31.5-52.7 63-52.7c.7 0 1.4 0 2.1 0c13.2-14.7 32.3-24 53.7-24c2.7 0 5.4 .2 8.1 .5c.1-2.8 .1-5.6 .1-8.5C512 114.6 397.4 0 256 0S0 114.6 0 256S114.6 512 256 512zM195.2 238.4c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2c17.6 23.5 52.8 23.5 70.4 0zm121.6 0c17.6 23.5 52.8 23.5 70.4 0c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2zM256 344a40 40 0 1 1 0 80 40 40 0 1 1 0-80zm287.4-5.5c.1-.8 .1-1.7 .1-2.5c0-22.1-17.9-40-39.9-40c-16.9 0-31.4 10.5-37.2 25.4c-3.3-.9-6.9-1.4-10.5-1.4c-22.1 0-39.9 17.9-39.9 40l-56.1 14.1c-4.6 1.1-7.8 5.2-7.8 9.9s3.2 8.8 7.8 9.9l56.9 14.2c-.5 2.5-.8 5.1-.8 7.8c0 22.1 17.9 40 39.9 40c4.9 0 9.6-.9 14-2.5c7.1 11.1 19.5 18.5 33.7 18.5c22.1 0 39.9-17.9 39.9-40c0-.8 0-1.7-.1-2.5c19-6.4 32.6-24.4 32.6-45.5s-13.6-39.1-32.6-45.5z";
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
    exports.faFaceExhaling = exports.definition;
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

// src/_entries/faFaceExhaling.js
var import_faFaceExhaling = __toESM(require_faFaceExhaling());
var export_faFaceExhaling = import_faFaceExhaling.definition;
export {
  export_faFaceExhaling as faFaceExhaling
};
