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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSunglasses.js
var require_faFaceSunglasses = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSunglasses.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-sunglasses";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e398";
    var svgPathData = "M256 512c141.4 0 256-114.6 256-256c0-16.6-1.6-32.8-4.6-48.5l2.6-15.7c7.9-47.6-20.5-91.8-63.3-106.6C399.8 32.9 331.8 0 256 0S112.2 32.9 65.3 85.2C22.4 100-6 144.2 2 191.8l2.6 15.7C1.6 223.2 0 239.4 0 256C0 397.4 114.6 512 256 512zM183.3 272l-78.7 0c-23.5 0-43.5-17-47.3-40.1l-8-48C44.4 154.6 67 128 96.7 128l94.7 0c14.4 0 27.1 6.2 35.8 16l57.8 0c8.7-9.8 21.4-16 35.8-16l94.7 0c29.7 0 52.2 26.6 47.3 55.9l-8 48C450.8 255 430.8 272 407.3 272l-78.7 0c-23.5 0-43.5-17-47.3-40.1l-8-48c-.4-2.7-.7-5.3-.7-7.9l-33.3 0c0 2.6-.2 5.2-.7 7.9l-8 48C226.8 255 206.8 272 183.3 272zm-19.2 69.5C182 362.2 212.6 384 256 384s74-21.8 91.9-42.5c5.8-6.7 15.9-7.4 22.6-1.6s7.4 15.9 1.6 22.6C349.8 388.1 311.1 416 256 416s-93.8-27.9-116.1-53.5c-5.8-6.7-5.1-16.8 1.6-22.6s16.8-5.1 22.6 1.6z";
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
    exports.faFaceSunglasses = exports.definition;
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

// src/_entries/faFaceSunglasses.js
var import_faFaceSunglasses = __toESM(require_faFaceSunglasses());
var export_faFaceSunglasses = import_faFaceSunglasses.definition;
export {
  export_faFaceSunglasses as faFaceSunglasses
};
