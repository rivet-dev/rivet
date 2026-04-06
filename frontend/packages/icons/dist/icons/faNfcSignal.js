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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faNfcSignal.js
var require_faNfcSignal = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faNfcSignal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "nfc-signal";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1fb";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 131c-22.4 3.4-43.8 9.6-64 18.3L384 144c0-26.5-21.5-48-48-48l-80 0c-26.5 0-48 21.5-48 48l0 66.7c-18.6 6.6-32 24.4-32 45.3c0 26.5 21.5 48 48 48s48-21.5 48-48c0-20.9-13.4-38.7-32-45.3l0-66.7c0-8.8 7.2-16 16-16l80 0c8.8 0 16 7.2 16 16l0 117.7c-47.2 28.4-84.8 71.2-106.7 122.3L112 384c-8.8 0-16-7.2-16-16l0-224c0-8.8 7.2-16 16-16l32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0c-26.5 0-48 21.5-48 48l0 224c0 26.5 21.5 48 48 48l121.9 0c-5.8 20.4-9.2 41.9-9.8 64L64 480c-35.3 0-64-28.7-64-64L0 96zM488 256c13.3 0 24 10.7 24 24s-10.7 24-24 24c-101.6 0-184 82.4-184 184c0 13.3-10.7 24-24 24s-24-10.7-24-24c0-128.1 103.9-232 232-232zm-8 192a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm32-72c0 13.3-10.7 24-24 24c-48.6 0-88 39.4-88 88c0 13.3-10.7 24-24 24s-24-10.7-24-24c0-75.1 60.9-136 136-136c13.3 0 24 10.7 24 24z";
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
    exports.faNfcSignal = exports.definition;
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

// src/_entries/faNfcSignal.js
var import_faNfcSignal = __toESM(require_faNfcSignal());
var export_faNfcSignal = import_faNfcSignal.definition;
export {
  export_faNfcSignal as faNfcSignal
};
