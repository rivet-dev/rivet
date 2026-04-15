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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBalloon.js
var require_faBalloon = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBalloon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "balloon";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e2e3";
    var svgPathData = "M0 192C0 86 86 0 192 0S384 86 384 192c0 102.6-102.8 194.9-143.6 227.5c-9.3 7.4-12.5 20.4-7.1 31l21.4 42.7c.9 1.8 1.4 3.8 1.4 5.8c0 7.1-5.8 12.9-12.9 12.9l-102.1 0c-7.1 0-12.9-5.8-12.9-12.9c0-2 .5-4 1.4-5.8l21.4-42.7c5.3-10.6 2.1-23.6-7.1-31C102.8 386.9 0 294.6 0 192zm96-16c0-44.2 35.8-80 80-80c8.8 0 16-7.2 16-16s-7.2-16-16-16C114.1 64 64 114.1 64 176c0 8.8 7.2 16 16 16s16-7.2 16-16z";
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
    exports.faBalloon = exports.definition;
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

// src/_entries/faBalloon.js
var import_faBalloon = __toESM(require_faBalloon());
var export_faBalloon = import_faBalloon.definition;
export {
  export_faBalloon as faBalloon
};
