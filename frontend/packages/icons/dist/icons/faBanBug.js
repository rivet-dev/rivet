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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBanBug.js
var require_faBanBug = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBanBug.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ban-bug";
    var width = 512;
    var height = 512;
    var aliases = ["debug"];
    var unicode = "f7f9";
    var svgPathData = "M256 448C150 448 64 362 64 256c0-41.5 13.1-79.9 35.5-111.2L367.2 412.5C335.9 434.9 297.5 448 256 448zm89-148.3l19.9 4c8.7 1.7 17.1-3.9 18.8-12.6s-3.9-17.1-12.6-18.8l-35.1-7 0-18.6 35.1-7c8.7-1.7 14.3-10.2 12.6-18.8s-10.2-14.3-18.8-12.6l-29.5 5.9c-1.1-9.2-3.8-17.8-7.7-25.8l24.5-14.7c7.6-4.5 10-14.4 5.5-22s-14.4-10-22-5.5L307.8 163c-14-11.9-32.1-19-51.8-19c-18.7 0-35.9 6.4-49.6 17.2L144.8 99.5C176.1 77.1 214.5 64 256 64c106 0 192 86 192 192c0 41.5-13.1 79.9-35.5 111.2L345 299.7zM256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM140.9 272.3c-8.7 1.7-14.3 10.2-12.6 18.8s10.2 14.3 18.8 12.6l29.5-5.9c1.1 9.2 3.8 17.8 7.7 25.8l-24.5 14.7c-7.6 4.5-10 14.4-5.5 22s14.4 10 22 5.5L204.2 349c14 11.9 32.1 19 51.8 19c6.6 0 13-.8 19.2-2.3L175 265.5l-34.1 6.8z";
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
    exports.faBanBug = exports.definition;
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

// src/_entries/faBanBug.js
var import_faBanBug = __toESM(require_faBanBug());
var export_faBanBug = import_faBanBug.definition;
export {
  export_faBanBug as faBanBug
};
