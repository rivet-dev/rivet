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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStandardDefinition.js
var require_faStandardDefinition = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStandardDefinition.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "standard-definition";
    var width = 576;
    var height = 512;
    var aliases = ["rectangle-sd"];
    var unicode = "e28a";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l448 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm304 88l0 144c0 13.3 10.7 24 24 24l56 0c53 0 96-43 96-96s-43-96-96-96l-56 0c-13.3 0-24 10.7-24 24zm48 24l32 0c26.5 0 48 21.5 48 48s-21.5 48-48 48l-32 0 0-96zm-184.9 .1c7.4-2.4 19.5-2.9 36.2-.7l3-22.2-3 22.2c5.1 .7 21.1 3.4 25.7 4.4c12.1 2.8 24.1-4.8 26.8-16.9s-4.8-24.1-16.9-26.8c-6.5-1.5-23.9-4.4-29.7-5.1c-18.9-2.5-38.9-3.1-56 2.5c-18.8 6.1-33.9 19.7-38.5 42.3c-.1 .4-.1 .8-.2 1.2c-1.7 11.4-.2 22.4 5 32.3c5.1 9.6 12.9 16.1 20.5 20.7c13.7 8.2 31.5 12.3 45.1 15.5c0 0 0 0 0 0l1.7 .4c16 3.7 27.1 6.5 34.1 10.7c3 1.8 3.8 3 3.9 3.3c0 0 0 0 0 0c.1 .1 .7 1.2 .2 4.7c-.4 2.7-1.2 3.8-1.7 4.3c-.7 .7-2.3 2-5.8 3.1c-7.7 2.4-20.1 2.9-36.9 .6c-7.4-1-20.3-4.8-31.4-8c-2.5-.7-5-1.4-7.2-2.1c-11.9-3.4-24.3 3.5-27.7 15.4s3.5 24.3 15.4 27.7c1.5 .4 3.3 .9 5.3 1.5c0 0 0 0 0 0c10.8 3.2 28.4 8.3 39.4 9.9l.1 0c18.9 2.5 39.2 3.1 56.4-2.3c8.9-2.8 17.8-7.5 24.9-15.2c7.3-7.8 11.8-17.6 13.4-28.5c1.7-11.4 .2-22.4-5-32.3c-5.1-9.6-12.9-16.1-20.5-20.7c-13.7-8.2-31.5-12.3-45.1-15.5c0 0 0 0 0 0l-1.7-.4c-16-3.7-27.1-6.5-34.1-10.7c-3-1.8-3.8-3-3.9-3.3c0 0 0 0 0 0c-.1-.1-.6-1.1-.3-4.1c.9-4 2.8-6.2 8.3-8z";
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
    exports.faStandardDefinition = exports.definition;
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

// src/_entries/faStandardDefinition.js
var import_faStandardDefinition = __toESM(require_faStandardDefinition());
var export_faStandardDefinition = import_faStandardDefinition.definition;
export {
  export_faStandardDefinition as faStandardDefinition
};
