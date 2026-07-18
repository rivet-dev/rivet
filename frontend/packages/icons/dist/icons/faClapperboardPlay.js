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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faClapperboardPlay.js
var require_faClapperboardPlay = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faClapperboardPlay.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "clapperboard-play";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e132";
    var svgPathData = "M453.8 32.3L327 159l-1 1-92.1 0L361 33l1-1L448 32c2 0 3.9 .1 5.8 .3zM393.9 160L498 56c8.8 11 14 24.9 14 40l0 64-118.1 0zM201.9 32l92.1 0L167 159l-1 1-92.1 0L201 33l1-1zm-67.9 0L7 159l-1 1L0 160 0 96C0 60.7 28.7 32 64 32l70.1 0zM0 192l512 0 0 224c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 192zm216.5 65.9c-5.2 2.8-8.5 8.2-8.5 14.1l0 128c0 5.9 3.2 11.3 8.5 14.1s11.5 2.5 16.4-.8l96-64c4.4-3 7.1-8 7.1-13.3s-2.7-10.3-7.1-13.3l-96-64c-4.9-3.3-11.2-3.6-16.4-.8z";
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
    exports.faClapperboardPlay = exports.definition;
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

// src/_entries/faClapperboardPlay.js
var import_faClapperboardPlay = __toESM(require_faClapperboardPlay());
var export_faClapperboardPlay = import_faClapperboardPlay.definition;
export {
  export_faClapperboardPlay as faClapperboardPlay
};
