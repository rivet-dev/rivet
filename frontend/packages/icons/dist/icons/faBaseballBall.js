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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBaseball.js
var require_faBaseball = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBaseball.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "baseball";
    var width = 512;
    var height = 512;
    var aliases = [129358, 9918, "baseball-ball"];
    var unicode = "f433";
    var svgPathData = "M232.4 1.1c-122.2 11.4-219.5 108.7-230.9 230.9 16.2-.1 32-1.8 47.2-5 13-2.7 25.7 5.5 28.4 18.5s-5.5 25.7-18.5 28.4c-18.4 3.9-37.6 6-57.1 6.1 11.4 122.2 108.7 219.5 230.9 230.9 .1-19.5 2.2-38.7 6.1-57.1 2.7-13 15.5-21.3 28.4-18.5s21.3 15.5 18.5 28.4c-3.2 15.2-4.9 31-5 47.2 122.2-11.4 219.5-108.7 230.9-230.9-16.2 .1-32 1.8-47.2 5-13 2.7-25.7-5.5-28.4-18.5s5.5-25.7 18.5-28.4c18.4-3.9 37.6-6 57.1-6.1-11.4-122.2-108.7-219.5-230.9-230.9-.1 19.5-2.2 38.7-6.1 57.1-2.7 13-15.5 21.3-28.4 18.5s-21.3-15.5-18.5-28.4c3.2-15.2 4.9-31 5-47.2zm2.8 151.4c-21.4 32.9-49.5 60.9-82.3 82.3-11.1 7.2-26 4.1-33.2-7s-4.1-26 7-33.2c27.2-17.7 50.5-41 68.3-68.3 7.2-11.1 22.1-14.3 33.2-7s14.3 22.1 7 33.2zM393.1 284.2c7.2 11.1 4.1 26-7 33.2-27.2 17.7-50.5 41-68.3 68.3-7.2 11.1-22.1 14.3-33.2 7s-14.3-22.1-7-33.2c21.4-32.9 49.5-60.9 82.3-82.3 11.1-7.2 26-4.1 33.2 7z";
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
    exports.faBaseball = exports.definition;
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

// src/_entries/faBaseballBall.js
var import_faBaseball = __toESM(require_faBaseball());
var export_faBaseballBall = import_faBaseball.definition;
export {
  export_faBaseballBall as faBaseballBall
};
