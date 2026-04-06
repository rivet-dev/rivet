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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceExplode.js
var require_faFaceExplode = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceExplode.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-explode";
    var width = 512;
    var height = 512;
    var aliases = ["exploding-head"];
    var unicode = "e2fe";
    var svgPathData = "M320 196.5c0 7.9 2.5 15.4 7 21.6c-14.5-6.6-31.5-5.6-45.3 3l-17.3 10.8c-5.2 3.2-11.8 3.2-17 0L230.2 221c-13.8-8.6-30.8-9.6-45.3-3c4.5-6.2 7-13.7 7-21.6l0-4.5c0-17.7-14.3-32-32-32l-40 0c-30.9 0-56-25.1-56-56s25.1-56 56-56c8.9 0 17.3 2.1 24.8 5.8C149.7 23.3 176.1 0 208 0c19.1 0 36.3 8.4 48 21.7C267.7 8.4 284.9 0 304 0c31.9 0 58.3 23.3 63.2 53.8c7.5-3.7 15.9-5.8 24.8-5.8c30.9 0 56 25.1 56 56s-25.1 56-56 56l-40 0c-17.7 0-32 14.3-32 32l0 4.5zM299 248c5.1-3 11.4-3 16.4 0c0 0 0 0 0 0c.1 .1 .2 .1 .3 .2L333 259c15.6 9.7 35.3 9.7 50.9 0l17.3-10.8c5.2-3.2 11.8-3.2 17 0l20.7 12.9c12.8 8 28.6 9.5 42.7 4.1L512 253.5c0 .8 0 1.7 0 2.5c0 141.4-114.6 256-256 256S0 397.4 0 256c0-.8 0-1.7 0-2.5l30.5 11.7c14.1 5.4 29.9 3.9 42.7-4.1l20.7-12.9c5.2-3.2 11.8-3.2 17 0L128.2 259c15.6 9.7 35.3 9.7 50.9 0l17.3-10.8c.1-.1 .2-.1 .3-.2c0 0 0 0 0 0c5.1-3 11.4-3 16.4 0c0 0 0 0 0 0c.1 .1 .2 .1 .3 .2L230.6 259c15.6 9.7 35.3 9.7 50.9 0l17.3-10.8c.1-.1 .2-.1 .3-.2c0 0 0 0 0 0zm-90.6 88a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm128 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM192 437.3c0 5.9 4.8 10.7 10.7 10.7l106.7 0c5.9 0 10.7-4.8 10.7-10.7c0-29.5-23.9-53.3-53.3-53.3l-21.3 0c-29.5 0-53.3 23.9-53.3 53.3z";
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
    exports.faFaceExplode = exports.definition;
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

// src/_entries/faExplodingHead.js
var import_faFaceExplode = __toESM(require_faFaceExplode());
var export_faExplodingHead = import_faFaceExplode.definition;
export {
  export_faExplodingHead as faExplodingHead
};
