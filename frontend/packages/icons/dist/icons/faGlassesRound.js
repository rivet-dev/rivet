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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGlassesRound.js
var require_faGlassesRound = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGlassesRound.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "glasses-round";
    var width = 576;
    var height = 512;
    var aliases = [128083, "glasses-alt"];
    var unicode = "f5f5";
    var svgPathData = "M118.6 80c-11.5 0-21.4 7.9-24 19.1L64.4 228.3C85.2 215.4 109.7 208 136 208c55.1 0 102.6 32.8 124 80l56.1 0c21.3-47.2 68.8-80 124-80c26.3 0 50.8 7.4 71.6 20.3L481.4 99.1c-2.6-11.2-12.6-19.1-24-19.1c-3.1 0-6.2 .6-9.2 1.8L416.9 94.3c-12.3 4.9-26.3-1.1-31.2-13.4s1.1-26.3 13.4-31.2l31.3-12.5c8.6-3.4 17.7-5.2 27-5.2c33.8 0 63.1 23.3 70.8 56.2l40.5 173.7c4.9 20.9 7.3 42.2 7.3 63.6l0 10.5c0 .8 0 1.6-.1 2.4c.1 1.9 .1 3.8 .1 5.6c0 75.1-60.9 136-136 136c-72.4 0-131.6-56.6-135.8-128l-32.5 0c-4.1 71.4-63.3 128-135.8 128C60.9 480 0 419.1 0 344c0-1.9 0-3.8 .1-5.6c-.1-.8-.1-1.6-.1-2.4l0-10.5c0-21.4 2.5-42.8 7.3-63.6L47.9 88.2C55.5 55.3 84.8 32 118.6 32c9.2 0 18.4 1.8 27 5.2l31.3 12.5c12.3 4.9 18.3 18.9 13.4 31.2s-18.9 18.3-31.2 13.4L127.8 81.8c-2.9-1.2-6-1.8-9.2-1.8zM136 416a72 72 0 1 0 0-144 72 72 0 1 0 0 144zm376-72a72 72 0 1 0 -144 0 72 72 0 1 0 144 0z";
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
    exports.faGlassesRound = exports.definition;
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

// src/_entries/faGlassesRound.js
var import_faGlassesRound = __toESM(require_faGlassesRound());
var export_faGlassesRound = import_faGlassesRound.definition;
export {
  export_faGlassesRound as faGlassesRound
};
