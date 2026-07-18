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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBanana.js
var require_faBanana = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBanana.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "banana";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e2e5";
    var svgPathData = "M304 144c0 35.7-7.3 69.6-20.5 100.5c4.8 3 9.4 6.2 13.9 9.8l35.5 28.4L413.9 261c10.6-2.8 21.4-4.4 32.3-4.9c1.2-10.5 1.8-21.3 1.8-32.1c0-84.7-36.6-160.9-94.8-213.6C345.6 3.6 335.9 0 326.1 0c-29.4 0-47.5 29.4-38 55c10.3 27.7 15.9 57.7 15.9 89zM277.4 279.2c-18.8-15-42.1-23.2-66.1-23.2c-22.9 0-45.2 7.4-63.5 21.2l-42.2 31.6c-10.6 8-12.8 23-4.8 33.6s23 12.8 33.6 4.8l42.2-31.6c10-7.5 22.2-11.6 34.7-11.6c10.5 0 20.8 2.9 29.7 8.2C194.1 366 125 400 48 400c-26.5 0-48 21.5-48 48l0 16c0 26.5 21.5 48 48 48l112 0c117.3 0 218.2-70.1 263.1-170.6l11.5-3.1c5.7-1.5 11.6-2.3 17.5-2.3c23.8 0 45.9 12.5 58.1 32.9l21.3 35.4c6.8 11.4 21.6 15 32.9 8.2s15-21.6 8.2-32.9l-21.3-35.4C530.4 309.3 492.7 288 452 288c-3.8 0-7.5 .2-11.3 .6c-5.5 .5-10.9 1.5-16.3 2.8c-.8 .2-1.5 .4-2.3 .6l-96.7 25.8s0 0 0 0l-48.1-38.5z";
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
    exports.faBanana = exports.definition;
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

// src/_entries/faBanana.js
var import_faBanana = __toESM(require_faBanana());
var export_faBanana = import_faBanana.definition;
export {
  export_faBanana as faBanana
};
