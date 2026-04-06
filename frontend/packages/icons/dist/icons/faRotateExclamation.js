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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRotateExclamation.js
var require_faRotateExclamation = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRotateExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rotate-exclamation";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e23c";
    var svgPathData = "M74.9 192L46.1 178.2c-7.6 15.9-.9 35 15 42.7s35 .9 42.7-15c0 0 0 0 0 0s0 0 0 0c0 0 0 0 0-.1l.2-.3 .7-1.5c.6-1.3 1.6-3.2 2.8-5.6c2.5-4.8 5.9-11.4 10-18.5c8.7-15.3 18.2-29.8 25.3-36.9c62.5-62.5 163.8-62.5 226.3 0L386.3 160 352 160c-17.7 0-32 14.3-32 32s14.3 32 32 32l112 0c17.7 0 32-14.3 32-32l0-112c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 35.2L414.4 97.6c-87.5-87.5-229.3-87.5-316.8 0c-13.8 13.8-27 35.2-35.7 50.6c-4.6 8.2-8.6 15.6-11.3 21c-1.4 2.7-2.5 4.9-3.3 6.5l-.9 1.8-.2 .5-.1 .2c0 0 0 0 0 0c0 0 0 0 0 0s0 0 0 0L74.9 192zM280 152c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 112c0 13.3 10.7 24 24 24s24-10.7 24-24l0-112zM256 384a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM48 464c17.7 0 32-14.3 32-32l0-35.2 17.6 17.6c87.5 87.5 229.3 87.5 316.8 0c24.4-24.4 42.1-53.1 52.9-83.7c5.9-16.7-2.9-34.9-19.5-40.8s-34.9 2.9-40.8 19.5c-7.7 21.8-20.2 42.3-37.8 59.8c-62.5 62.5-163.8 62.5-226.3 0L125.7 352l34.3 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L48 288c-17.7 0-32 14.3-32 32l0 112c0 17.7 14.3 32 32 32z";
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
    exports.faRotateExclamation = exports.definition;
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

// src/_entries/faRotateExclamation.js
var import_faRotateExclamation = __toESM(require_faRotateExclamation());
var export_faRotateExclamation = import_faRotateExclamation.definition;
export {
  export_faRotateExclamation as faRotateExclamation
};
