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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBasketballHoop.js
var require_faBasketballHoop = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBasketballHoop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "basketball-hoop";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f435";
    var svgPathData = "M6.8 187C24.7 153.1 116.7 0 320 0S615.3 153.1 633.2 187c4.9 9.4 6.8 19.1 6.8 28l0 116.3c0 25.3-14.9 48.2-38 58.5l-96.4 42.9L526.2 288l-32.3 0-7.2 50.3-22.9 160c-.9 6.1-5.2 11.2-11.1 13s-12.3 .1-16.5-4.4L384.4 451l-52.8 56c-3 3.2-7.2 5-11.6 5s-8.6-1.8-11.6-5l-52.8-56-51.9 55.9c-4.2 4.5-10.6 6.2-16.5 4.4s-10.2-6.9-11.1-13l-22.9-160L146.1 288l-32.3 0 20.7 144.6L38 389.7C14.9 379.4 0 356.5 0 331.2L0 214.9c0-8.9 1.8-18.6 6.8-28zM178.4 288l5.2 36.6 44-36.6-49.2 0zm99.3 0l-.6 .5L320 334.6l42.8-46.1-.6-.5-84.4 0zm134.5 0l44 36.6 5.2-36.6-49.2 0zM192 128c-17.7 0-32 14.3-32 32l0 48-48 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0 288 0 64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-48 0 0-48c0-17.7-14.3-32-32-32l-256 0zm256 32l0 48-256 0 0-48 256 0zM387.5 309l-45.7 49.2 42.7 46 51.6-54.7L387.5 309zm49.4 151.6l11-76.8-41.5 44 30.5 32.9zM320 381.7l-42.6 45.9L320 472.7l42.6-45.2L320 381.7zm-21.8-23.5L252.5 309l-48.7 40.5 51.6 54.7 42.7-46zm-64.6 69.5l-41.5-44 11 76.8 30.5-32.9z";
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
    exports.faBasketballHoop = exports.definition;
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

// src/_entries/faBasketballHoop.js
var import_faBasketballHoop = __toESM(require_faBasketballHoop());
var export_faBasketballHoop = import_faBasketballHoop.definition;
export {
  export_faBasketballHoop as faBasketballHoop
};
