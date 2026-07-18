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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle1.js
var require_faCircle1 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle1.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-1";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0ee";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zM268 131.2c-7.4-4.3-16.5-4.3-24-.1l-56 32c-11.5 6.6-15.5 21.2-8.9 32.7s21.2 15.5 32.7 8.9L232 193.4 232 336l-40 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l64 0 64 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-40 0 0-184c0-8.6-4.6-16.5-12-20.8z";
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
    exports.faCircle1 = exports.definition;
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

// src/_entries/faCircle1.js
var import_faCircle1 = __toESM(require_faCircle1());
var export_faCircle1 = import_faCircle1.definition;
export {
  export_faCircle1 as faCircle1
};
