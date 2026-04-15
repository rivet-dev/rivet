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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBat.js
var require_faBat = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bat";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f6b5";
    var svgPathData = "M235.6 52.8l10.8 14.4c6 8.1 15.5 12.8 25.6 12.8l32 0c10.1 0 19.6-4.7 25.6-12.8l10.8-14.4c2.3-3 5.8-4.8 9.6-4.8c5.8 0 10.8 4.1 11.8 9.8l19 101.4L452.6 125c16.2-7.7 35.5-3.6 47.1 10C548.9 192.4 576 265.6 576 341.2l0 37.6c0 9.7-10.2 16-18.9 11.7l-63.8-31.9c-7.5-3.7-16.6-1.1-20.9 6.1l-29.8 49.7c-5.4 8.9-17.7 10.4-25 3.1l-36.6-36.6c-6.8-6.8-18.1-6.1-24 1.6l-56.2 73.1c-6.4 8.3-19 8.3-25.4 0l-56.2-73.1c-5.9-7.7-17.2-8.4-24-1.6l-36.6 36.6c-7.4 7.4-19.7 5.8-25-3.1l-29.8-49.7c-4.3-7.2-13.4-9.8-20.9-6.1L18.9 390.5c-8.7 4.3-18.9-2-18.9-11.7l0-37.6C0 265.6 27.1 192.4 76.3 135c11.6-13.6 30.9-17.7 47.1-10l71.8 34.2 19-101.4c1.1-5.7 6-9.8 11.8-9.8c3.8 0 7.4 1.8 9.6 4.8z";
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
    exports.faBat = exports.definition;
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

// src/_entries/faBat.js
var import_faBat = __toESM(require_faBat());
var export_faBat = import_faBat.definition;
export {
  export_faBat as faBat
};
