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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTransporter7.js
var require_faTransporter7 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTransporter7.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "transporter-7";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2a8";
    var svgPathData = "M472.7 38.6c-1.1-3.9-4.7-6.6-8.7-6.6s-7.6 2.7-8.7 6.6L448 64l-25.4 7.3c-3.9 1.1-6.6 4.7-6.6 8.7s2.7 7.6 6.6 8.7L448 96l7.3 25.4c1.1 3.9 4.7 6.6 8.7 6.6s7.6-2.7 8.7-6.6L480 96l25.4-7.3c3.9-1.1 6.6-4.7 6.6-8.7s-2.7-7.6-6.6-8.7L480 64l-7.3-25.4zM64 288l-7.3-25.4c-1.1-3.9-4.7-6.6-8.7-6.6s-7.6 2.7-8.7 6.6L32 288 6.6 295.3C2.7 296.4 0 299.9 0 304s2.7 7.6 6.6 8.7L32 320l7.3 25.4c1.1 3.9 4.7 6.6 8.7 6.6s7.6-2.7 8.7-6.6L64 320l25.4-7.3c3.9-1.1 6.6-4.7 6.6-8.7s-2.7-7.6-6.6-8.7L64 288zM296 512l88 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-256 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l88 0 80 0z";
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
    exports.faTransporter7 = exports.definition;
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

// src/_entries/faTransporter7.js
var import_faTransporter7 = __toESM(require_faTransporter7());
var export_faTransporter7 = import_faTransporter7.definition;
export {
  export_faTransporter7 as faTransporter7
};
