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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWavesSine.js
var require_faWavesSine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWavesSine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "waves-sine";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e65d";
    var svgPathData = "M256.2 140.6c-11 18.2-20.4 40.1-29.3 64.6c-9.2-27.9-19.8-57.4-32.9-84.7c2.4-4.5 4.9-8.9 7.5-13.2C226.3 66.4 263.3 32 320 32s93.7 34.4 118.6 75.4c23.8 39.2 40.3 90.2 55.2 136.5l.7 2.3c15.8 49.1 30.2 93.1 49.7 125.2C563 402.4 582.6 416 608 416c17.7 0 32 14.3 32 32s-14.3 32-32 32c-56.7 0-93.7-34.4-118.6-75.4c-23.8-39.2-40.3-90.2-55.2-136.5l-.7-2.3c-15.8-49.1-30.2-93.1-49.7-125.2C365 109.6 345.4 96 320 96s-45 13.6-63.8 44.6zM158.1 391.4c-2.4 4.5-4.9 8.9-7.5 13.2C125.7 445.6 88.7 480 32 480c-17.7 0-32-14.3-32-32s14.3-32 32-32c25.4 0 45-13.6 63.8-44.6c11-18.2 20.4-40.1 29.3-64.6c9.2 27.9 19.8 57.4 32.9 84.7zm225.8-20c11-18.2 20.4-40.1 29.3-64.6c9.2 27.9 19.8 57.4 32.9 84.7c-2.4 4.5-4.9 8.9-7.5 13.2C413.7 445.6 376.7 480 320 480s-93.7-34.4-118.6-75.4c-23.8-39.2-40.3-90.2-55.2-136.5c0 0 0 0 0 0l-.7-2.3c-15.8-49.1-30.2-93.1-49.7-125.2C77 109.6 57.4 96 32 96C14.3 96 0 81.7 0 64S14.3 32 32 32c56.7 0 93.7 34.4 118.6 75.4c23.8 39.2 40.3 90.2 55.2 136.5l.7 2.3c15.8 49.1 30.2 93.1 49.7 125.2C275 402.4 294.6 416 320 416s45-13.6 63.8-44.6zM544.2 140.6c-11 18.2-20.4 40.1-29.3 64.6c-9.2-27.9-19.8-57.5-32.9-84.7c2.4-4.5 4.9-8.9 7.5-13.2C514.3 66.4 551.3 32 608 32c17.7 0 32 14.3 32 32s-14.3 32-32 32c-25.4 0-45 13.6-63.8 44.6z";
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
    exports.faWavesSine = exports.definition;
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

// src/_entries/faWavesSine.js
var import_faWavesSine = __toESM(require_faWavesSine());
var export_faWavesSine = import_faWavesSine.definition;
export {
  export_faWavesSine as faWavesSine
};
