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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAxeBattle.js
var require_faAxeBattle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAxeBattle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "axe-battle";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6b3";
    var svgPathData = "M95.9 14l0 .1c0 .1 .1 .4 .1 .7c.1 .7 .3 1.8 .6 3.3c.6 3 1.7 7.4 3.5 12.8c3.6 10.8 9.9 25.1 20.6 39.5c13.9 18.6 36 37.9 71.2 48.8l0 147.6c-29.9 11.1-50.9 29.3-65.6 47.3c-11.9 14.6-19.7 29.3-24.5 40.3c-2.4 5.5-4 10-5.1 13.1c-.5 1.5-.9 2.7-1.1 3.5l-.2 .8 0 .1s0 0 0 0c-1.5 5.8-6.1 10.3-11.9 11.7s-12-.7-15.8-5.3L80 368c-12.2 10.3-12.2 10.3-12.2 10.3s0 0 0 0s0 0 0 0c0 0 0 0-.1-.1l-.2-.2-.6-.8c-.5-.7-1.3-1.6-2.3-2.9c-1.9-2.5-4.6-6.1-7.7-10.6c-6.3-9.2-14.7-22.4-23.1-38.9C17 291.9 0 245.3 0 192S17 92.1 33.7 59.3c8.4-16.5 16.8-29.7 23.1-38.9c3.2-4.6 5.8-8.2 7.7-10.6c1-1.2 1.7-2.2 2.3-2.9l.6-.8 .2-.2c0 0 0 0 .1-.1c0 0 0 0 0 0s0 0 0 0L80 16 67.8 5.7c4.1-4.9 10.7-6.8 16.8-5c6.1 1.8 10.5 7 11.3 13.3c0 0 0 0 0 0zM320 266.8l0-147.6c35.2-10.9 57.3-30.2 71.2-48.8C403.3 54.3 412.8 34 416.1 14c.8-6.3 5.3-11.5 11.3-13.3c6.1-1.8 12.8 .3 16.9 5.2c13.3 16.3 24.4 34.8 33.9 53.4c12.4 24.5 25.1 56.6 30.7 93.4c.5 3.5-1.4 7-4.7 8.4l-55.1 23.6c-6.5 2.8-6.5 11.9 0 14.7L504.3 223c3.3 1.4 5.3 4.8 4.7 8.4c-8 53-31.3 105.5-64.5 146.6c-3.9 4.6-10.3 7-16.1 5.6s-10.5-6-11.9-11.8c-5.9-21-17.2-40.9-30.9-57.7c-14.7-18-35.7-36.2-65.6-47.3zM288 64l0 416c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-416c0-17.7 14.3-32 32-32s32 14.3 32 32z";
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
    exports.faAxeBattle = exports.definition;
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

// src/_entries/faAxeBattle.js
var import_faAxeBattle = __toESM(require_faAxeBattle());
var export_faAxeBattle = import_faAxeBattle.definition;
export {
  export_faAxeBattle as faAxeBattle
};
