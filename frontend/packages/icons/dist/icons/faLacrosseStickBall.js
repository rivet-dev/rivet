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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLacrosseStickBall.js
var require_faLacrosseStickBall = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLacrosseStickBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lacrosse-stick-ball";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e3b6";
    var svgPathData = "M336 64s0 0 0 0l.3 0c19.9 0 48.1 13.6 73.1 38.6s38.6 53.2 38.6 73.1c0 .1 0 .2 0 .3c0 0 0 0 0 0c0 33.1-21.9 62.1-53.7 71.2l-11.5 3.3c-19.6 5.6-37.9 15.1-53.9 27.8L294 306.2c-11.1 8.9-25 13.8-39.2 13.8c-34.7 0-62.8-28.1-62.8-62.8c0-14.3 4.9-28.1 13.8-39.2l27.9-34.9c12.8-15.9 22.2-34.3 27.8-53.9l3.3-11.5C273.9 85.9 302.9 64 336 64zm0-64s0 0 0 0C274.4 0 220.2 40.9 203.3 100.1L200 111.7c-3.3 11.5-8.8 22.2-16.2 31.5l-27.9 34.9c-18 22.5-27.8 50.4-27.8 79.2c0 23.4 6.3 45.3 17.4 64.1l-136 136c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0l136-136c18.8 11.1 40.7 17.4 64.1 17.4c28.8 0 56.7-9.8 79.2-27.8l34.9-27.9c9.3-7.4 20-13 31.5-16.2l11.5-3.3C471.1 291.8 512 237.6 512 176c0 0 0 0 0 0l0-.3c0-43.8-26.4-87.4-57.3-118.4S380.1 0 336.3 0L336 0zM496 512a80 80 0 1 0 0-160 80 80 0 1 0 0 160z";
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
    exports.faLacrosseStickBall = exports.definition;
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

// src/_entries/faLacrosseStickBall.js
var import_faLacrosseStickBall = __toESM(require_faLacrosseStickBall());
var export_faLacrosseStickBall = import_faLacrosseStickBall.definition;
export {
  export_faLacrosseStickBall as faLacrosseStickBall
};
