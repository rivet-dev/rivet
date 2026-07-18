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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMobileVibrate.js
var require_faMobileVibrate = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMobileVibrate.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mobile-vibrate";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e816";
    var svgPathData = "M384 0c35.3 0 64 28.7 64 64l0 384c0 35.3-28.7 64-64 64l-192 0c-35.3 0-64-28.7-64-64l0-384c0-35.3 28.7-64 64-64L384 0zM248 416c-13.3 0-24 10.7-24 24s10.7 24 24 24l80 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-80 0zM48.8 133.4c9.4-7.7 23.3-7.1 32.1 1.7 9.4 9.4 9.4 24.6 0 33.9l-31 31 25.4 25.4 2.9 3.2c6.3 7.7 9.8 17.4 9.8 27.4 0 11.5-4.6 22.5-12.7 30.6l-25.4 25.4 31 31 1.7 1.8c7.7 9.4 7.1 23.3-1.7 32.1s-22.7 9.3-32.1 1.7L47 377 12.7 342.6C5.6 335.5 1.2 326.2 .2 316.3L0 312 .2 307.7c1-9.9 5.4-19.2 12.5-26.3L38.1 256 12.7 230.6C5.6 223.5 1.2 214.2 .2 204.3L0 200 .2 195.7c1-9.9 5.4-19.2 12.5-26.3L47 135 48.8 133.4zM495 135c8.8-8.8 22.7-9.3 32.1-1.7l1.8 1.7 34.3 34.3c7.1 7.1 11.5 16.4 12.5 26.3l.2 4.3-.2 4.3c-1 9.9-5.4 19.2-12.5 26.3l-25.4 25.4 25.4 25.4c7.1 7.1 11.5 16.4 12.5 26.3l.2 4.3-.2 4.3c-1 9.9-5.4 19.2-12.5 26.3l-34.3 34.3-1.8 1.7c-9.4 7.7-23.3 7.1-32.1-1.7s-9.3-22.7-1.7-32.1l1.7-1.8 31-31-25.4-25.4c-8.1-8.1-12.7-19.1-12.7-30.6 0-10 3.5-19.7 9.8-27.4l2.9-3.2 25.4-25.4-31-31c-9.4-9.4-9.4-24.6 0-33.9z";
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
    exports.faMobileVibrate = exports.definition;
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

// src/_entries/faMobileVibrate.js
var import_faMobileVibrate = __toESM(require_faMobileVibrate());
var export_faMobileVibrate = import_faMobileVibrate.definition;
export {
  export_faMobileVibrate as faMobileVibrate
};
