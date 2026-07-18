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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle2.js
var require_faCircle2 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle2.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-2";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0ef";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM222.7 184.7l-24.2 18.4c-10.5 8-25.6 6-33.6-4.5s-6-25.6 4.5-33.6l24.2-18.4c15.8-12 35.2-18.4 55.1-18.1l3.4 .1c46.5 .7 83.8 38.6 83.8 85.1c0 23.5-9.7 46-26.9 62.1L244.7 336l83.3 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-144 0c-9.8 0-18.7-6-22.3-15.2s-1.3-19.6 5.9-26.3L276.3 240.6c7.5-7 11.7-16.8 11.7-27.1c0-20.3-16.3-36.8-36.6-37.1l-3.4-.1c-9.1-.1-18 2.8-25.3 8.3z";
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
    exports.faCircle2 = exports.definition;
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

// src/_entries/faCircle2.js
var import_faCircle2 = __toESM(require_faCircle2());
var export_faCircle2 = import_faCircle2.definition;
export {
  export_faCircle2 as faCircle2
};
