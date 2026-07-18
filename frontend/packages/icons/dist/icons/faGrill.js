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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGrill.js
var require_faGrill = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGrill.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "grill";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e5a3";
    var svgPathData = "M0 72C0 49.9 17.9 32 40 32l368 0c22.1 0 40 17.9 40 40l0 24c0 89.8-52.9 167.3-129.2 203l63.2 147.5c5.2 12.2-.4 26.3-12.6 31.5s-26.3-.4-31.5-12.6L316.7 416 160 416c0 35.3-28.7 64-64 64s-64-28.7-64-64s28.7-64 64-64c3.4 0 6.8 .3 10.1 .8L129.2 299C52.9 263.3 0 185.8 0 96L0 72zM151.8 368l144.3 0-22.9-53.4c-15.9 3.6-32.3 5.4-49.3 5.4s-33.4-1.9-49.3-5.4L151.8 368zM96 440a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faGrill = exports.definition;
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

// src/_entries/faGrill.js
var import_faGrill = __toESM(require_faGrill());
var export_faGrill = import_faGrill.definition;
export {
  export_faGrill as faGrill
};
