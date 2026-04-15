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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMicrophoneLinesSlash.js
var require_faMicrophoneLinesSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMicrophoneLinesSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "microphone-lines-slash";
    var width = 576;
    var height = 512;
    var aliases = ["microphone-alt-slash"];
    var unicode = "f539";
    var svgPathData = "M7-24.9c9.4-9.4 24.6-9.4 33.9 0l151 151 0-30.2c0-53 43-96 96-96 50.3 0 91.6 38.7 95.7 88L328 88c-13.3 0-24 10.7-24 24s10.7 24 24 24l56 0 0 48-56 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l55.7 0c-1.9 22.9-11.8 43.4-26.9 59l33.9 33.9c25.5-26 41.3-61.6 41.3-100.9l0-40c0-13.3 10.7-24 24-24s24 10.7 24 24l0 40c0 52.6-21.1 100.2-55.3 134.8L569 503.1c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0L7 9.1C-2.3-.3-2.3-15.5 7-24.9zM297.8 367.7l41.4 41.4c-8.8 2.4-17.9 4.3-27.3 5.4l0 49.5 48 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-144 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l48 0 0-49.5C169.3 402.7 96 321.9 96 224l0-40c0-5.1 1.6-9.9 4.4-13.8l43.6 43.6 0 10.2c0 79.5 64.5 144 144 144 3.3 0 6.6-.1 9.8-.3z";
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
    exports.faMicrophoneLinesSlash = exports.definition;
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

// src/_entries/faMicrophoneLinesSlash.js
var import_faMicrophoneLinesSlash = __toESM(require_faMicrophoneLinesSlash());
var export_faMicrophoneLinesSlash = import_faMicrophoneLinesSlash.definition;
export {
  export_faMicrophoneLinesSlash as faMicrophoneLinesSlash
};
