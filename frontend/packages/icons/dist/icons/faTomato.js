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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTomato.js
var require_faTomato = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTomato.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tomato";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e330";
    var svgPathData = "M241 7L224 24C241 7 241 7 241 7s0 0 0 0s0 0 0 0l.1 .1 .2 .2 .5 .5c.4 .4 1 1 1.7 1.8c1.4 1.5 3.2 3.6 5.4 6.3c4.4 5.4 10.1 13.1 15.8 22.8c6.6 11.2 13.4 25.5 18 42.1c45.7 3.1 88.5 14.2 125.1 32.8c6.6 3.4 10 10.8 8.3 18s-8.2 12.3-15.6 12.3l-34.6 0 15.4 23.1c3.9 5.9 3.5 13.7-1 19.1s-12.1 7.3-18.6 4.5L256 145.4 150.3 190.7c-6.5 2.8-14.1 1-18.6-4.5s-4.9-13.2-1-19.1l15.5-23.3-31.3-.2c-7 1.5-14.3-1.7-17.7-8.4c-4-7.9-.9-17.5 7-21.5l.8-.4C142.2 94.6 185.6 83.5 232 80.7c-2.6-6.4-5.6-12.3-8.7-17.6c-4.3-7.3-8.6-13-11.7-16.9c-1.6-1.9-2.8-3.4-3.6-4.3c-.4-.4-.7-.8-.9-.9l-.1-.1c-9.3-9.4-9.3-24.5 .1-33.9c9.4-9.4 24.6-9.4 33.9 0zM71.1 154c6 9.4 14.9 16.1 25 19.5c-.6 11.8 3.1 23.7 11 33.2c13.6 16.3 36.3 21.8 55.8 13.4L256 180.2l93.1 39.9c19.5 8.4 42.2 2.9 55.8-13.4c7.9-9.5 11.7-21.5 11-33.2c10.3-3.5 19.1-10.4 25-19.5c44 35.5 71.1 86.4 71.1 150c0 130.9-114.6 208-256 208S0 434.9 0 304c0-63.6 27.1-114.5 71.1-150z";
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
    exports.faTomato = exports.definition;
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

// src/_entries/faTomato.js
var import_faTomato = __toESM(require_faTomato());
var export_faTomato = import_faTomato.definition;
export {
  export_faTomato as faTomato
};
