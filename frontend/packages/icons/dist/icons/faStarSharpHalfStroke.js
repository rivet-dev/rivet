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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarSharpHalfStroke.js
var require_faStarSharpHalfStroke = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarSharpHalfStroke.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "star-sharp-half-stroke";
    var width = 576;
    var height = 512;
    var aliases = ["star-sharp-half-alt"];
    var unicode = "e28d";
    var svgPathData = "M56 230.4L41.3 218.9c-8.1-6.3-11.3-17-8-26.7S45.7 176 56 176l18.7 0 2.5 0 .1 0 137.2 0L258.6 37.2l.1-.3 1.3-4s0 0 0 0l5.2-16.2C268.3 6.8 277.6 0 288 0s19.7 6.8 22.9 16.7L316 32.9l1.3 4 .1 .3L361.6 176l137.2 0 .1 0 2.5 0 18.7 0c10.3 0 19.4 6.5 22.7 16.2s.1 20.4-8 26.7L520 230.4l-2 1.5-.1 .1-114 88.7 44.4 139.5 .1 .4 1.4 4.5 5 15.7c3.2 10-.5 21-9.1 27s-20.2 5.7-28.5-.7l-13-10.1-3.7-2.9-.3-.3L288 406.4 175.8 493.7l-.3 .3-3.7 2.9-13 10.1c-8.3 6.5-19.8 6.8-28.5 .7s-12.3-16.9-9.1-27l5-15.7 1.4-4.5 .1-.4 44.4-139.5L58.1 232l-.1-.1-2-1.5zM288 335.5s0 0 0 0l34.4 26.7 47.9 37.2-19.7-61.8-11.8-37.2 30.8-24L426.7 232l-65.2 0-41 0-12.4-39L288 129.5s0 0 0 0l0 205.9z";
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
    exports.faStarSharpHalfStroke = exports.definition;
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

// src/_entries/faStarSharpHalfStroke.js
var import_faStarSharpHalfStroke = __toESM(require_faStarSharpHalfStroke());
var export_faStarSharpHalfStroke = import_faStarSharpHalfStroke.definition;
export {
  export_faStarSharpHalfStroke as faStarSharpHalfStroke
};
