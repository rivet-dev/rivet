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

// src/node_modules/@fortawesome/free-solid-svg-icons/faKaaba.js
var require_faKaaba = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faKaaba.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "kaaba";
    var width = 512;
    var height = 512;
    var aliases = [128331];
    var unicode = "f66b";
    var svgPathData = "M256 51.3L92.8 112.4 247.5 171.1c5.5 2.1 11.5 2.1 17 0L419.2 112.4 256 51.3zM0 129.3c0-20 12.4-37.9 31.1-44.9l208-78c10.9-4.1 22.8-4.1 33.7 0l208 78c18.7 7 31.1 24.9 31.1 44.9l0 36-253.2 96c-1.8 .7-3.8 .7-5.7 0l-253.2-96 0-36zm0 140l0-52.7 236.1 89.6c12.8 4.9 26.9 4.9 39.7 0l236.1-89.6 0 52.7-128 48.6 0 51.3 128-48.6 0 62.2c0 20-12.4 37.9-31.1 44.9l-208 78c-10.9 4.1-22.8 4.1-33.7 0l-208-78C12.4 420.7 0 402.7 0 382.7l0-62.2 128 48.6 0-51.3-128-48.6zM236.1 410.1c12.8 4.9 26.9 4.9 39.7 0l60.1-22.8 0-51.3-77.2 29.3c-1.8 .7-3.8 .7-5.7 0l-77.2-29.3 0 51.3 60.1 22.8z";
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
    exports.faKaaba = exports.definition;
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

// src/_entries/faKaaba.js
var import_faKaaba = __toESM(require_faKaaba());
var export_faKaaba = import_faKaaba.definition;
export {
  export_faKaaba as faKaaba
};
