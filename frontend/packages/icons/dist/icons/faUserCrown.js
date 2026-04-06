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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserCrown.js
var require_faUserCrown = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserCrown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-crown";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f6a4";
    var svgPathData = "M144 128l0 16c0 44.2 35.8 80 80 80s80-35.8 80-80l0-16-160 0zm0-108.8c10.2 7.5 23.8 8.3 34.9 2L209.1 4c4.6-2.6 9.7-4 14.9-4s10.4 1.4 14.9 4l30.2 17.2c11 6.3 24.7 5.5 34.9-2l.1-.1c.3-.2 .6-.4 .8-.6l3-2.4L323.6 3.5c2.8-2.3 6.4-3.5 10-3.5L336 0c8.8 0 16 7.2 16 16l0 23 0 3.2c0 0 0 .1 0 .1L352 144c0 70.7-57.3 128-128 128s-128-57.3-128-128L96 42.3c0 0 0-.1 0-.1L96 39l0-23c0-8.8 7.2-16 16-16l2.4 0c3.6 0 7.2 1.2 10 3.5L140 16l3 2.4c.3 .2 .6 .4 .8 .6l.1 .1zM0 472c0-92.8 75.2-168 168-168l112 0c92.8 0 168 75.2 168 168l0 8c0 17.7-14.3 32-32 32L32 512c-17.7 0-32-14.3-32-32l0-8z";
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
    exports.faUserCrown = exports.definition;
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

// src/_entries/faUserCrown.js
var import_faUserCrown = __toESM(require_faUserCrown());
var export_faUserCrown = import_faUserCrown.definition;
export {
  export_faUserCrown as faUserCrown
};
