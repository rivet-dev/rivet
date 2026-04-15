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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFrenchFries.js
var require_faFrenchFries = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFrenchFries.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "french-fries";
    var width = 384;
    var height = 512;
    var aliases = [127839];
    var unicode = "f803";
    var svgPathData = "M183.2 1.7c-5-2.5-10.8-2.2-15.6 .7S160 10.5 160 16l0 190.7s0 0 0 0l0 63.2c9.2 1.4 19.8 2.2 32 2.2s22.8-.8 32-2.2l0-238c-.1-6-3.5-11.4-8.8-14.1l-32-16zM128 14.7l-.2-1.3c-.8-5-4-9.3-8.5-11.6s-9.9-2.3-14.4-.1l-32 16c-6.3 3.1-9.8 10-8.6 16.9l32 192c.5 1.2 .9 2.3 1.2 3.4c2 6.3 6.7 15.9 18.7 24.3c3.2 2.2 7.1 4.5 11.8 6.6l0-246.3zM40.3 85.6L19.9 80.5c-5.3-1.3-10.8 .1-14.8 3.8S-.7 93.4 .2 98.8l16.5 93.4c1.6-.1 3.1-.2 4.7-.2L48 192c3.6 0 7 .4 10.2 1L40.3 85.6zM383.8 98.8c.9-5.3-.9-10.8-4.8-14.5s-9.5-5.1-14.8-3.8L346.2 85l-19 107.8c2.8-.5 5.7-.8 8.8-.8l26.6 0c1.6 0 3.2 .1 4.7 .2l16.5-93.4zm-64-48c.9-5.3-.9-10.8-4.8-14.5s-9.5-5.1-14.8-3.8l-32 8C262 42 257.3 47 256.2 53.2l-.2 1.4L256 261c4.8-2.1 8.7-4.4 11.8-6.6c12-8.4 16.7-18 18.7-24.3c.7-2.3 1.7-4.7 2.8-7L319.8 50.8zM.6 250.5l57.5 237C61.6 501.9 74.4 512 89.2 512l205.7 0c14.8 0 27.6-10.1 31.1-24.5l57.5-237c.4-1.7 .6-3.3 .6-5.1c0-11.8-9.6-21.4-21.4-21.4L336 224c-8.8 0-16.3 7.2-18.9 15.7C309.7 263 283.7 304 192 304s-117.7-41-125.1-64.3C64.3 231.2 56.8 224 48 224l-26.6 0C9.6 224 0 233.6 0 245.4c0 1.7 .2 3.4 .6 5.1z";
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
    exports.faFrenchFries = exports.definition;
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

// src/_entries/faFrenchFries.js
var import_faFrenchFries = __toESM(require_faFrenchFries());
var export_faFrenchFries = import_faFrenchFries.definition;
export {
  export_faFrenchFries as faFrenchFries
};
