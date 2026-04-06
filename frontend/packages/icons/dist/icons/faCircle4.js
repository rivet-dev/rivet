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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle4.js
var require_faCircle4 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle4.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-4";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0f1";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zM223.6 129.2c-12.6-4.2-26.2 2.6-30.4 15.2l-48 144c-2.4 7.3-1.2 15.4 3.3 21.6s11.8 10 19.5 10l104 0 0 40c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40 8 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-8 0 0-56c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 56-70.7 0 37.5-112.4c4.2-12.6-2.6-26.2-15.2-30.4z";
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
    exports.faCircle4 = exports.definition;
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

// src/_entries/faCircle4.js
var import_faCircle4 = __toESM(require_faCircle4());
var export_faCircle4 = import_faCircle4.definition;
export {
  export_faCircle4 as faCircle4
};
