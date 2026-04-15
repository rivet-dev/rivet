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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLibra.js
var require_faLibra = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLibra.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "libra";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e84b";
    var svgPathData = "M480 384c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 448c-17.7 0-32-14.3-32-32s14.3-32 32-32l448 0zM256 32c97.2 0 176 78.8 176 176 0 16.6-2.4 32.7-6.7 48l54.7 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-104.2 0c-11.8 0-22.7-6.5-28.2-16.9s-4.9-23.1 1.6-32.9c11.9-17.8 18.8-39.1 18.8-62.2 0-61.9-50.1-112-112-112S144 146.1 144 208c0 23.1 6.9 44.4 18.8 62.2 6.6 9.8 7.2 22.5 1.6 32.9S148.1 320 136.2 320L32 320c-17.7 0-32-14.3-32-32s14.3-32 32-32l54.8 0c-4.3-15.3-6.8-31.4-6.8-48 0-97.2 78.8-176 176-176z";
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
    exports.faLibra = exports.definition;
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

// src/_entries/faLibra.js
var import_faLibra = __toESM(require_faLibra());
var export_faLibra = import_faLibra.definition;
export {
  export_faLibra as faLibra
};
