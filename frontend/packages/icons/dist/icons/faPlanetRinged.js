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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPlanetRinged.js
var require_faPlanetRinged = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPlanetRinged.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "planet-ringed";
    var width = 512;
    var height = 512;
    var aliases = [129680];
    var unicode = "e020";
    var svgPathData = "M456.1 31.1c0 0-.1 .5 0 1.2c.1-.9 0-1.3 0-1.2zM414.7 83.9c10.7-6.4 20.4-11.5 28.9-15.5c-4.1 8.5-9.2 18.2-15.5 28.9c-4.4 7.4-9.2 15.1-14.4 23.1c-6.8-7.9-14.1-15.2-22-22c8-5.2 15.7-10.1 23.1-14.4zM65.7 340.2C10.4 418.3-14 481.7 8.1 503.9c35.2 35.2 174.7-47.3 311.6-184.2S539.1 43.3 503.9 8.1C481.7-14 418.3 10.4 340.2 65.7C314.4 54.3 286 48 256 48C141.1 48 48 141.1 48 256c0 30 6.3 58.4 17.7 84.2zm32.6 51.5c6.8 7.9 14.1 15.2 22 22c-8 5.2-15.7 10.1-23.1 14.4c-10.7 6.4-20.4 11.5-28.9 15.5c4.1-8.5 9.2-18.2 15.5-28.9c4.4-7.4 9.2-15.1 14.4-23.1zM31.1 456.1c0 0 .4 .1 1.2 0c-.8-.1-1.2 0-1.2 0zM56 479.7c-.1 .9 0 1.3 0 1.2s.1-.5 0-1.2zM479.7 56c.8 .1 1.2 0 1.2 0s-.4-.1-1.2 0zM256 464c114.9 0 208-93.1 208-208c0-16.1-1.8-31.8-5.3-46.9c-31.8 42.6-71.4 88.2-116.4 133.2s-90.6 84.6-133.2 116.4c15.1 3.5 30.7 5.3 46.9 5.3z";
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
    exports.faPlanetRinged = exports.definition;
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

// src/_entries/faPlanetRinged.js
var import_faPlanetRinged = __toESM(require_faPlanetRinged());
var export_faPlanetRinged = import_faPlanetRinged.definition;
export {
  export_faPlanetRinged as faPlanetRinged
};
