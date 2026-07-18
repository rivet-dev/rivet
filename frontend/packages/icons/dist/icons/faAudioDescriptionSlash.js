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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAudioDescriptionSlash.js
var require_faAudioDescriptionSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAudioDescriptionSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "audio-description-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e0a8";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-30-23.5c4.6-8.8 7.2-18.9 7.2-29.6l0-320c0-35.3-28.7-64-64-64L96 32c-6.7 0-13.1 1-19.2 2.9L38.8 5.1zM336 238l0-54c0-13.3 10.7-24 24-24l56 0c53 0 96 43 96 96c0 35.3-19 66.1-47.4 82.8l-44.6-35c24.7-2 44.1-22.7 44.1-47.8c0-26.5-21.5-48-48-48l-32 0 0 67.7L336 238zM487.4 480L314.3 343.6c-2 2.4-4.6 4.4-7.5 5.9c-11.9 5.9-26.3 1.1-32.2-10.7l-9.4-18.9-82.2 0-9.4 18.9c-5.9 11.9-20.3 16.7-32.2 10.7s-16.7-20.3-10.7-32.2l42.5-85L32 121.2 32 416c0 35.3 28.7 64 64 64l391.4 0zm-254-200.2l-21.8-17.2-8.6 17.2 30.4 0z";
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
    exports.faAudioDescriptionSlash = exports.definition;
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

// src/_entries/faAudioDescriptionSlash.js
var import_faAudioDescriptionSlash = __toESM(require_faAudioDescriptionSlash());
var export_faAudioDescriptionSlash = import_faAudioDescriptionSlash.definition;
export {
  export_faAudioDescriptionSlash as faAudioDescriptionSlash
};
