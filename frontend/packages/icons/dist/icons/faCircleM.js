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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleM.js
var require_faCircleM = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleM.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-m";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e115";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM171.5 137.9L256 255l84.5-117.1c6.1-8.4 16.9-12 26.8-8.8s16.6 12.4 16.6 22.8l0 208c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-133.8-60.5 83.8c-4.5 6.2-11.7 9.9-19.5 9.9s-14.9-3.7-19.5-9.9L176 226.2 176 360c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-208c0-10.4 6.7-19.6 16.6-22.8s20.7 .3 26.8 8.8z";
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
    exports.faCircleM = exports.definition;
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

// src/_entries/faCircleM.js
var import_faCircleM = __toESM(require_faCircleM());
var export_faCircleM = import_faCircleM.definition;
export {
  export_faCircleM as faCircleM
};
