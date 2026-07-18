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

// src/node_modules/@fortawesome/free-solid-svg-icons/faScrollTorah.js
var require_faScrollTorah = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faScrollTorah.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scroll-torah";
    var width = 576;
    var height = 512;
    var aliases = ["torah"];
    var unicode = "f6a0";
    var svgPathData = "M0 48C0 21.5 21.5 0 48 0 68.9 0 86.7 13.4 93.3 32l389.5 0c6.6-18.6 24.4-32 45.3-32 26.5 0 48 21.5 48 48l0 416c0 26.5-21.5 48-48 48-20.9 0-38.7-13.4-45.3-32L93.3 480c-6.6 18.6-24.4 32-45.3 32-26.5 0-48-21.5-48-48L0 48zM177.9 301c-1.2 2-1.9 4.2-1.9 6.6 0 6.9 5.6 12.5 12.5 12.5l56.9 0 30.5 49.2c2.6 4.2 7.2 6.8 12.2 6.8s9.6-2.6 12.2-6.8l30.5-49.2 56.9 0c6.9 0 12.5-5.6 12.5-12.5 0-2.3-.6-4.6-1.9-6.6l-27.8-45 27.8-45c1.2-2 1.9-4.2 1.9-6.6 0-6.9-5.6-12.5-12.5-12.5l-56.9 0-30.5-49.2c-2.6-4.2-7.2-6.8-12.2-6.8s-9.6 2.6-12.2 6.8l-30.5 49.2-56.9 0c-6.9 0-12.5 5.6-12.5 12.5 0 2.3 .6 4.6 1.9 6.6l27.8 45-27.8 45z";
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
    exports.faScrollTorah = exports.definition;
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

// src/_entries/faScrollTorah.js
var import_faScrollTorah = __toESM(require_faScrollTorah());
var export_faScrollTorah = import_faScrollTorah.definition;
export {
  export_faScrollTorah as faScrollTorah
};
